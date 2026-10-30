//! 数据验证：inspect 名单，apply 写 `dataValidations`。

use serde_json::{json, Map, Value};

use super::a1::{self, CellRef};
use super::layout;
use super::request::{
    check_dv_fields, DeleteDataValidationOp, DvOperator, DvType, InsertDataValidationOp, Patch,
    SetDataValidationOp,
};
use crate::error::{Error, ErrorKind};
use crate::xml::{Element, Node};

pub(crate) fn count(root: &Element) -> usize {
    root.child_by_local("dataValidations")
        .map(|p| p.children_by_local("dataValidation").len())
        .unwrap_or(0)
}

pub(crate) fn inspect(root: &Element, sheet: &str) -> Value {
    let Some(parent) = root.child_by_local("dataValidations") else {
        return json!([]);
    };
    let mut out = Vec::new();
    for (i, el) in parent
        .children_by_local("dataValidation")
        .into_iter()
        .enumerate()
    {
        out.push(inspect_one(el, sheet, i + 1));
    }
    Value::Array(out)
}

fn inspect_one(el: &Element, sheet: &str, n: usize) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("sheet:{sheet}/dv:{n}")));
    obj.insert(
        "sqref".into(),
        inspect_sqref(el.attr("sqref").unwrap_or("")),
    );
    let type_ = el.attr("type").unwrap_or("none");
    obj.insert("type".into(), json!(type_));
    if let Some(op) = el.attr("operator") {
        obj.insert("operator".into(), json!(op));
    }
    if let Some(f1) = el.child_by_local("formula1") {
        obj.insert("formula1".into(), json!(f1.text_content()));
    }
    if let Some(f2) = el.child_by_local("formula2") {
        obj.insert("formula2".into(), json!(f2.text_content()));
    }
    obj.insert(
        "allowBlank".into(),
        json!(crate::xml::attr_is_true_trimmed(el, "allowBlank")),
    );
    if type_ == "list" {
        // XML showDropDown 为真表示隐藏下拉，与 JSON dropdown 相反。
        obj.insert(
            "dropdown".into(),
            json!(!crate::xml::attr_is_true_trimmed(el, "showDropDown")),
        );
    }
    obj.insert(
        "showInputMessage".into(),
        json!(crate::xml::attr_is_true_trimmed(el, "showInputMessage")),
    );
    obj.insert(
        "showErrorMessage".into(),
        json!(crate::xml::attr_is_true_trimmed(el, "showErrorMessage")),
    );
    if let Some(v) = el.attr("errorStyle") {
        obj.insert("errorStyle".into(), json!(v));
    }
    if let Some(v) = el.attr("errorTitle") {
        obj.insert("errorTitle".into(), json!(v));
    }
    if let Some(v) = el.attr("error") {
        obj.insert("error".into(), json!(v));
    }
    if let Some(v) = el.attr("promptTitle") {
        obj.insert("promptTitle".into(), json!(v));
    }
    if let Some(v) = el.attr("prompt") {
        obj.insert("prompt".into(), json!(v));
    }
    Value::Object(obj)
}

fn inspect_sqref(raw: &str) -> Value {
    let parts: Vec<Value> = raw
        .split_whitespace()
        .map(|tok| match a1::parse_range(tok) {
            Ok((a, b)) => json!(a1::format_range(a, b)),
            Err(_) => json!(tok),
        })
        .collect();
    Value::Array(parts)
}

pub(crate) fn apply_insert(
    worksheet: &mut Element,
    op: &InsertDataValidationOp,
    _i: usize,
) -> Result<(), Error> {
    let mut el = Element::new("dataValidation");
    write_new_rule(&mut el, op);
    let parent = ensure_data_validations(worksheet);
    parent.push_element(el);
    set_count(parent);
    Ok(())
}

pub(crate) fn apply_set(
    worksheet: &mut Element,
    op: &SetDataValidationOp,
    i: usize,
) -> Result<(), Error> {
    let el = nth_mut(worksheet, op.index, i, &op.sheet)?;
    apply_patch(el, op, i)?;
    Ok(())
}

pub(crate) fn apply_delete(
    worksheet: &mut Element,
    op: &DeleteDataValidationOp,
    i: usize,
) -> Result<(), Error> {
    let Some(pos) = worksheet
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "dataValidations"))
    else {
        return Err(not_found(&op.sheet, op.index, i));
    };
    let Node::Element(parent) = &mut worksheet.children[pos] else {
        return Err(not_found(&op.sheet, op.index, i));
    };
    let mut seen = 0usize;
    let before = parent.children.len();
    parent.children.retain(|n| match n {
        Node::Element(e) if e.local_name() == "dataValidation" => {
            seen += 1;
            seen != op.index
        }
        _ => true,
    });
    if parent.children.len() == before {
        return Err(not_found(&op.sheet, op.index, i));
    }
    if parent.children_by_local("dataValidation").is_empty() {
        worksheet.children.remove(pos);
        return Ok(());
    }
    set_count(parent);
    Ok(())
}

fn write_new_rule(el: &mut Element, op: &InsertDataValidationOp) {
    if op.type_ != DvType::None {
        el.set_attr("type", op.type_.as_str());
    }
    if let Some(opv) = op.operator {
        if opv != DvOperator::Between {
            el.set_attr("operator", opv.as_str());
        }
    }
    set_bool_attr(el, "allowBlank", op.allow_blank);
    if op.type_ == DvType::List {
        // dropdown true：不写 showDropDown；false：写 1 表示隐藏。
        if op.dropdown == Some(false) {
            el.set_attr("showDropDown", "1");
        }
    }
    set_bool_attr(el, "showInputMessage", op.show_input_message);
    set_bool_attr(el, "showErrorMessage", op.show_error_message);
    if let Some(s) = &op.error_style {
        if s != "stop" {
            el.set_attr("errorStyle", s);
        }
    }
    set_opt_attr(el, "errorTitle", op.error_title.as_deref());
    set_opt_attr(el, "error", op.error.as_deref());
    set_opt_attr(el, "promptTitle", op.prompt_title.as_deref());
    set_opt_attr(el, "prompt", op.prompt.as_deref());
    el.set_attr("sqref", format_sqref(&op.sqref));
    if let Some(f) = &op.formula1 {
        el.push_element(Element::new("formula1").with_text(prepare_formula(op.type_, f)));
    }
    if let Some(f) = &op.formula2 {
        el.push_element(Element::new("formula2").with_text(f));
    }
}

fn apply_patch(el: &mut Element, op: &SetDataValidationOp, i: usize) -> Result<(), Error> {
    let mut type_ = el
        .attr("type")
        .and_then(DvType::parse)
        .unwrap_or(DvType::None);
    if let Some(t) = op.type_ {
        type_ = t;
    }
    let mut operator = match &op.operator {
        Patch::Absent => el.attr("operator").and_then(DvOperator::parse),
        Patch::Clear => None,
        Patch::Set(v) => Some(*v),
    };
    let mut formula1 = match &op.formula1 {
        Patch::Absent => formula_text(el, "formula1"),
        Patch::Clear => None,
        Patch::Set(s) => Some(s.clone()),
    };
    let mut formula2 = match &op.formula2 {
        Patch::Absent => formula_text(el, "formula2"),
        Patch::Clear => None,
        Patch::Set(s) => Some(s.clone()),
    };
    let mut dropdown = op.dropdown;
    if !type_.allows_operator() {
        if matches!(op.operator, Patch::Set(_)) {
            return Err(Error::with_path(
                ErrorKind::InvalidDataValidation {
                    field: "operator".into(),
                },
                format!("ops[{i}].operator"),
            ));
        }
        operator = None;
    }
    if !type_.allows_operator() {
        if matches!(op.formula2, Patch::Set(_)) {
            return Err(Error::with_path(
                ErrorKind::InvalidDataValidation {
                    field: "formula2".into(),
                },
                format!("ops[{i}].formula2"),
            ));
        }
        formula2 = None;
    }
    if type_ != DvType::List {
        if dropdown.is_some() {
            return Err(Error::with_path(
                ErrorKind::InvalidDataValidation {
                    field: "dropdown".into(),
                },
                format!("ops[{i}].dropdown"),
            ));
        }
        dropdown = None;
    } else if dropdown.is_none() {
        dropdown = Some(!crate::xml::attr_is_true_trimmed(el, "showDropDown"));
    }
    if type_ == DvType::List {
        if let Some(f) = &formula1 {
            formula1 = Some(prepare_formula(type_, f));
        }
    }
    check_dv_fields(
        type_,
        operator,
        formula1.as_deref(),
        formula2.as_deref(),
        if type_ == DvType::List {
            dropdown
        } else {
            None
        },
        i,
    )?;

    write_type_attr(el, type_);
    write_operator_attr(el, type_, operator);
    if let Some(ranges) = &op.sqref {
        el.set_attr("sqref", format_sqref(ranges));
    }
    if let Some(b) = op.allow_blank {
        set_bool_attr(el, "allowBlank", b);
    }
    if type_ == DvType::List {
        if dropdown == Some(false) {
            el.set_attr("showDropDown", "1");
        } else {
            el.remove_attr("showDropDown");
        }
    } else {
        el.remove_attr("showDropDown");
    }
    if let Some(b) = op.show_input_message {
        set_bool_attr(el, "showInputMessage", b);
    }
    if let Some(b) = op.show_error_message {
        set_bool_attr(el, "showErrorMessage", b);
    }
    match &op.error_style {
        Patch::Absent => {}
        Patch::Clear => {
            el.remove_attr("errorStyle");
        }
        Patch::Set(s) => {
            if s == "stop" {
                el.remove_attr("errorStyle");
            } else {
                el.set_attr("errorStyle", s);
            }
        }
    }
    patch_attr(el, "errorTitle", &op.error_title);
    patch_attr(el, "error", &op.error);
    patch_attr(el, "promptTitle", &op.prompt_title);
    patch_attr(el, "prompt", &op.prompt);
    write_formula_child(el, "formula1", formula1.as_deref());
    write_formula_child(el, "formula2", formula2.as_deref());
    Ok(())
}

fn write_type_attr(el: &mut Element, type_: DvType) {
    if type_ == DvType::None {
        el.remove_attr("type");
    } else {
        el.set_attr("type", type_.as_str());
    }
}

fn write_operator_attr(el: &mut Element, type_: DvType, operator: Option<DvOperator>) {
    if !type_.allows_operator() {
        el.remove_attr("operator");
        return;
    }
    match operator {
        None | Some(DvOperator::Between) => el.remove_attr("operator"),
        Some(op) => el.set_attr("operator", op.as_str()),
    }
}

fn write_formula_child(el: &mut Element, local: &str, text: Option<&str>) {
    if let Some(t) = text {
        if let Some(child) = el.child_by_local_mut(local) {
            child.set_text(t);
            return;
        }
        let child = Element::new(local).with_text(t);
        insert_formula(el, child, local);
        return;
    }
    el.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != local,
        _ => true,
    });
}

fn insert_formula(parent: &mut Element, child: Element, local: &str) {
    let want = if local == "formula1" { 0 } else { 1 };
    let mut at = parent.children.len();
    let mut seen = 0usize;
    for (i, n) in parent.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if e.local_name() == "formula1" || e.local_name() == "formula2" {
            if seen == want {
                at = i;
                break;
            }
            seen += 1;
        }
    }
    parent.children.insert(at, Node::Element(child));
}

fn formula_text(el: &Element, local: &str) -> Option<String> {
    el.child_by_local(local)
        .map(|c| c.text_content())
        .filter(|s| !s.is_empty())
}

fn prepare_formula(type_: DvType, raw: &str) -> String {
    if type_ != DvType::List {
        return raw.to_string();
    }
    if raw.starts_with('"') {
        return raw.to_string();
    }
    if raw.contains('(') || raw.contains('!') {
        return raw.to_string();
    }
    if a1::parse_range(raw).is_ok() {
        return raw.to_string();
    }
    format!("\"{raw}\"")
}

fn format_sqref(ranges: &[(CellRef, CellRef)]) -> String {
    ranges
        .iter()
        .map(|(a, b)| a1::format_range(*a, *b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn ensure_data_validations(worksheet: &mut Element) -> &mut Element {
    if worksheet.child_by_local("dataValidations").is_none() {
        layout::insert_child_ordered(
            worksheet,
            Element::new("dataValidations"),
            "dataValidations",
        );
    }
    worksheet.child_by_local_mut("dataValidations").unwrap()
}

fn set_count(parent: &mut Element) {
    let n = parent.children_by_local("dataValidation").len();
    parent.set_attr("count", n.to_string());
}

fn nth_mut<'a>(
    worksheet: &'a mut Element,
    index: usize,
    i: usize,
    sheet: &str,
) -> Result<&'a mut Element, Error> {
    let Some(parent) = worksheet.child_by_local_mut("dataValidations") else {
        return Err(not_found(sheet, index, i));
    };
    let mut seen = 0usize;
    let mut found = None;
    for (idx, n) in parent.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        if e.local_name() != "dataValidation" {
            continue;
        }
        seen += 1;
        if seen == index {
            found = Some(idx);
            break;
        }
    }
    let Some(idx) = found else {
        return Err(not_found(sheet, index, i));
    };
    match &mut parent.children[idx] {
        Node::Element(e) => Ok(e),
        _ => Err(not_found(sheet, index, i)),
    }
}

fn not_found(sheet: &str, index: usize, i: usize) -> Error {
    Error::with_path(
        ErrorKind::DataValidationNotFound {
            id: format!("sheet:{sheet}/dv:{index}"),
        },
        format!("ops[{i}].id"),
    )
}

fn set_bool_attr(el: &mut Element, name: &str, on: bool) {
    if on {
        el.set_attr(name, "1");
    } else {
        el.remove_attr(name);
    }
}

fn set_opt_attr(el: &mut Element, name: &str, v: Option<&str>) {
    if let Some(s) = v {
        el.set_attr(name, s);
    }
}

fn patch_attr(el: &mut Element, name: &str, p: &Patch<String>) {
    match p {
        Patch::Absent => {}
        Patch::Clear => el.remove_attr(name),
        Patch::Set(s) => el.set_attr(name, s),
    }
}
