//! 条件格式：inspect 名单，apply 写 `conditionalFormatting` 与 `dxfs`。

use serde_json::{json, Map, Value};

use super::a1::{self, CellRef};
use super::layout;
use super::request::{
    check_cf_fields, icon_set_count, CfOperator, CfType, CfvoSpec, ColorScaleSpec, ColorSpec,
    DataBarSpec, DeleteConditionalFormatOp, IconSetSpec, InsertConditionalFormatOp, Patch,
    SetConditionalFormatOp,
};
use super::style;
use crate::error::{Error, ErrorKind};
use crate::xml::{Element, Node};

pub(crate) fn count(root: &Element) -> usize {
    root.children_by_local("conditionalFormatting")
        .into_iter()
        .map(|b| b.children_by_local("cfRule").len())
        .sum()
}

pub(crate) fn inspect(root: &Element, sheet: &str, styles: &style::Stylebook) -> Value {
    let mut out = Vec::new();
    let mut n = 0usize;
    for block in root.children_by_local("conditionalFormatting") {
        let sqref = inspect_sqref(block.attr("sqref").unwrap_or(""));
        for rule in block.children_by_local("cfRule") {
            n += 1;
            out.push(inspect_one(rule, sheet, n, sqref.clone(), styles));
        }
    }
    Value::Array(out)
}

fn inspect_one(
    el: &Element,
    sheet: &str,
    n: usize,
    sqref: Value,
    styles: &style::Stylebook,
) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("sheet:{sheet}/cf:{n}")));
    obj.insert("sqref".into(), sqref);
    if let Some(t) = el.attr("type") {
        obj.insert("type".into(), json!(t));
    }
    if let Some(p) = el.attr("priority").and_then(|s| s.parse::<i64>().ok()) {
        obj.insert("priority".into(), json!(p));
    }
    if let Some(op) = el.attr("operator") {
        obj.insert("operator".into(), json!(op));
    }
    let formulas: Vec<Value> = el
        .children_by_local("formula")
        .into_iter()
        .map(|f| json!(f.text_content()))
        .collect();
    if !formulas.is_empty() {
        obj.insert("formulas".into(), Value::Array(formulas));
    }
    if let Some(t) = el.attr("text") {
        obj.insert("text".into(), json!(t));
    }
    if let Some(t) = el.attr("timePeriod") {
        obj.insert("timePeriod".into(), json!(t));
    }
    if let Some(r) = el.attr("rank").and_then(|s| s.parse::<u64>().ok()) {
        obj.insert("rank".into(), json!(r));
    }
    if attr_true(el, "percent") {
        obj.insert("percent".into(), json!(true));
    }
    if attr_true(el, "bottom") {
        obj.insert("bottom".into(), json!(true));
    }
    if attr_true(el, "aboveAverage") {
        obj.insert("aboveAverage".into(), json!(true));
    }
    if attr_true(el, "equalAverage") {
        obj.insert("equalAverage".into(), json!(true));
    }
    if let Some(s) = el.attr("stdDev").and_then(|s| s.parse::<i64>().ok()) {
        obj.insert("stdDev".into(), json!(s));
    }
    if attr_true(el, "stopIfTrue") {
        obj.insert("stopIfTrue".into(), json!(true));
    }
    if let Some(id) = el.attr("dxfId").and_then(|s| s.parse::<usize>().ok()) {
        if let Some(dxf) = styles.dxf_json(id) {
            obj.insert("dxf".into(), dxf);
        }
    }
    if let Some(cs) = el.child_by_local("colorScale") {
        obj.insert("colorScale".into(), inspect_color_scale(cs));
    }
    if let Some(db) = el.child_by_local("dataBar") {
        obj.insert("dataBar".into(), inspect_data_bar(db));
    }
    if let Some(is) = el.child_by_local("iconSet") {
        obj.insert("iconSet".into(), inspect_icon_set(is));
    }
    Value::Object(obj)
}

fn inspect_color_scale(el: &Element) -> Value {
    let mut obj = Map::new();
    obj.insert("cfvo".into(), inspect_cfvo_list(el));
    let colors: Vec<Value> = el
        .children_by_local("color")
        .into_iter()
        .filter_map(style::color_json)
        .collect();
    obj.insert("colors".into(), Value::Array(colors));
    Value::Object(obj)
}

fn inspect_data_bar(el: &Element) -> Value {
    let mut obj = Map::new();
    obj.insert("cfvo".into(), inspect_cfvo_list(el));
    if let Some(c) = el.child_by_local("color").and_then(style::color_json) {
        obj.insert("color".into(), c);
    }
    if let Some(n) = el.attr("minLength").and_then(|s| s.parse::<u64>().ok()) {
        obj.insert("minLength".into(), json!(n));
    }
    if let Some(n) = el.attr("maxLength").and_then(|s| s.parse::<u64>().ok()) {
        obj.insert("maxLength".into(), json!(n));
    }
    if el.attr("showValue").is_some() && !attr_true(el, "showValue") {
        obj.insert("showValue".into(), json!(false));
    }
    Value::Object(obj)
}

fn inspect_icon_set(el: &Element) -> Value {
    let mut obj = Map::new();
    obj.insert(
        "set".into(),
        json!(el.attr("iconSet").unwrap_or("3TrafficLights1")),
    );
    obj.insert("cfvo".into(), inspect_cfvo_list(el));
    if el.attr("showValue").is_some() && !attr_true(el, "showValue") {
        obj.insert("showValue".into(), json!(false));
    }
    if attr_true(el, "reverse") {
        obj.insert("reverse".into(), json!(true));
    }
    if el.attr("percent").is_some() && !attr_true(el, "percent") {
        obj.insert("percent".into(), json!(false));
    }
    Value::Object(obj)
}

fn inspect_cfvo_list(el: &Element) -> Value {
    let items: Vec<Value> = el
        .children_by_local("cfvo")
        .into_iter()
        .map(inspect_cfvo)
        .collect();
    Value::Array(items)
}

fn inspect_cfvo(el: &Element) -> Value {
    let mut obj = Map::new();
    obj.insert("type".into(), json!(el.attr("type").unwrap_or("")));
    if let Some(v) = el.attr("val") {
        obj.insert("val".into(), json!(v));
    }
    if el.attr("gte").is_some() && !attr_true(el, "gte") {
        obj.insert("gte".into(), json!(false));
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
    op: &InsertConditionalFormatOp,
    dxf_id: Option<usize>,
    _i: usize,
) -> Result<(), Error> {
    let priority = op.priority.unwrap_or_else(|| next_priority(worksheet));
    let mut rule = Element::new("cfRule");
    write_rule(
        &mut rule,
        op.type_,
        op.operator,
        op.formulas.as_deref(),
        dxf_id,
        op.color_scale.as_ref(),
        op.data_bar.as_ref(),
        op.icon_set.as_ref(),
        priority,
        op.stop_if_true,
    );
    let mut block = Element::new("conditionalFormatting");
    block.set_attr("sqref", format_sqref(&op.sqref));
    block.push_element(rule);
    layout::insert_child_ordered(worksheet, block, "conditionalFormatting");
    Ok(())
}

pub(crate) fn apply_set(
    worksheet: &mut Element,
    op: &SetConditionalFormatOp,
    dxf_id: Option<Option<usize>>,
    i: usize,
) -> Result<(), Error> {
    let (cf_i, rule_i) =
        find_rule(worksheet, op.index).ok_or_else(|| not_found(&op.sheet, op.index, i))?;
    if let Some(ranges) = &op.sqref {
        let Node::Element(cf) = &mut worksheet.children[cf_i] else {
            return Err(not_found(&op.sheet, op.index, i));
        };
        cf.set_attr("sqref", format_sqref(ranges));
    }
    let Node::Element(cf) = &mut worksheet.children[cf_i] else {
        return Err(not_found(&op.sheet, op.index, i));
    };
    let Node::Element(rule) = &mut cf.children[rule_i] else {
        return Err(not_found(&op.sheet, op.index, i));
    };
    apply_rule_patch(rule, op, dxf_id, i)
}

pub(crate) fn apply_delete(
    worksheet: &mut Element,
    op: &DeleteConditionalFormatOp,
    i: usize,
) -> Result<(), Error> {
    let (cf_i, rule_i) =
        find_rule(worksheet, op.index).ok_or_else(|| not_found(&op.sheet, op.index, i))?;
    let Node::Element(cf) = &mut worksheet.children[cf_i] else {
        return Err(not_found(&op.sheet, op.index, i));
    };
    if rule_i >= cf.children.len() {
        return Err(not_found(&op.sheet, op.index, i));
    }
    cf.children.remove(rule_i);
    let empty = cf.children_by_local("cfRule").is_empty();
    if empty {
        worksheet.children.remove(cf_i);
    }
    Ok(())
}

fn apply_rule_patch(
    rule: &mut Element,
    op: &SetConditionalFormatOp,
    dxf_id: Option<Option<usize>>,
    i: usize,
) -> Result<(), Error> {
    let existing_type = rule.attr("type").unwrap_or("").to_string();
    let writable_now = CfType::is_writable(&existing_type);
    if !writable_now
        && op.type_.is_none()
        && (op.operator.is_present()
            || op.formulas.is_present()
            || op.color_scale.is_present()
            || op.data_bar.is_present()
            || op.icon_set.is_present())
    {
        return Err(Error::with_path(
            ErrorKind::InvalidConditionalFormat {
                field: "type".into(),
            },
            format!("ops[{i}].type"),
        ));
    }

    let type_ = if let Some(t) = op.type_ {
        t
    } else {
        CfType::parse(&existing_type).unwrap_or(CfType::CellIs)
    };
    let changing_type =
        op.type_.is_some() && op.type_.map(|t| t.as_str()) != Some(existing_type.as_str());

    if changing_type || (op.type_.is_some() && !writable_now) {
        clear_typed_children(rule);
        rule.remove_attr("operator");
        rule.remove_attr("text");
        rule.remove_attr("timePeriod");
        rule.remove_attr("rank");
        rule.remove_attr("percent");
        rule.remove_attr("bottom");
        rule.remove_attr("aboveAverage");
        rule.remove_attr("equalAverage");
        rule.remove_attr("stdDev");
        rule.set_attr("type", type_.as_str());
    }

    let operator = match &op.operator {
        Patch::Absent => rule.attr("operator").and_then(CfOperator::parse),
        Patch::Clear => {
            rule.remove_attr("operator");
            None
        }
        Patch::Set(v) => {
            rule.set_attr("operator", v.as_str());
            Some(*v)
        }
    };
    let formulas = match &op.formulas {
        Patch::Absent => Some(existing_formulas(rule)).filter(|v| !v.is_empty()),
        Patch::Clear => {
            remove_children(rule, "formula");
            None
        }
        Patch::Set(v) => {
            remove_children(rule, "formula");
            for f in v {
                rule.push_element(Element::new("formula").with_text(f.clone()));
            }
            Some(v.clone())
        }
    };
    match &op.color_scale {
        Patch::Absent => {}
        Patch::Clear => remove_children(rule, "colorScale"),
        Patch::Set(spec) => {
            remove_children(rule, "colorScale");
            rule.push_element(write_color_scale(spec));
        }
    }
    match &op.data_bar {
        Patch::Absent => {}
        Patch::Clear => remove_children(rule, "dataBar"),
        Patch::Set(spec) => {
            remove_children(rule, "dataBar");
            rule.push_element(write_data_bar(spec));
        }
    }
    match &op.icon_set {
        Patch::Absent => {}
        Patch::Clear => remove_children(rule, "iconSet"),
        Patch::Set(spec) => {
            remove_children(rule, "iconSet");
            rule.push_element(write_icon_set(spec));
        }
    }

    if let Some(p) = op.priority {
        rule.set_attr("priority", p.to_string());
    }
    if let Some(stop) = op.stop_if_true {
        if stop {
            rule.set_attr("stopIfTrue", "1");
        } else {
            rule.remove_attr("stopIfTrue");
        }
    }
    if let Some(opt) = dxf_id {
        match opt {
            Some(id) => rule.set_attr("dxfId", id.to_string()),
            None => rule.remove_attr("dxfId"),
        }
    }

    let color_scale = rule.child_by_local("colorScale").map(|_| ());
    // 改完之后仍须满足 insert 约束；非可写类型且未改 type 已在上面拒绝结构字段。
    if op.type_.is_some() || writable_now {
        let cs = match &op.color_scale {
            Patch::Set(s) => Some(s.clone()),
            Patch::Clear => None,
            Patch::Absent => existing_color_scale(rule),
        };
        let db = match &op.data_bar {
            Patch::Set(s) => Some(s.clone()),
            Patch::Clear => None,
            Patch::Absent => existing_data_bar(rule),
        };
        let iset = match &op.icon_set {
            Patch::Set(s) => Some(s.clone()),
            Patch::Clear => None,
            Patch::Absent => existing_icon_set(rule),
        };
        let dxf_present = rule.attr("dxfId").is_some();
        check_cf_fields(
            type_,
            operator,
            formulas.as_deref(),
            if dxf_present {
                Some(&super::request::DxfPatch {
                    font: None,
                    fill: None,
                    border: None,
                })
            } else {
                None
            },
            cs.as_ref(),
            db.as_ref(),
            iset.as_ref(),
            i,
            changing_type || op.type_.is_some(),
        )?;
        if type_ == CfType::ColorScale
            && color_scale.is_none()
            && !matches!(op.color_scale, Patch::Set(_))
        {
            return Err(Error::with_path(
                ErrorKind::MissingField {
                    field: "colorScale".into(),
                },
                format!("ops[{i}].colorScale"),
            ));
        }
        if (changing_type || op.type_.is_some())
            && type_ == CfType::ColorScale
            && rule.child_by_local("colorScale").is_none()
        {
            if let Patch::Set(spec) = &op.color_scale {
                rule.push_element(write_color_scale(spec));
            }
        }
        if (changing_type || op.type_.is_some())
            && type_ == CfType::DataBar
            && rule.child_by_local("dataBar").is_none()
        {
            let spec = match &op.data_bar {
                Patch::Set(s) => s.clone(),
                _ => DataBarSpec {
                    cfvo: None,
                    color: None,
                    min_length: None,
                    max_length: None,
                    show_value: None,
                },
            };
            rule.push_element(write_data_bar(&spec));
        }
        if (changing_type || op.type_.is_some())
            && type_ == CfType::IconSet
            && rule.child_by_local("iconSet").is_none()
        {
            let spec = match &op.icon_set {
                Patch::Set(s) => s.clone(),
                _ => IconSetSpec {
                    set: None,
                    cfvo: None,
                    show_value: None,
                    reverse: None,
                    percent: None,
                },
            };
            rule.push_element(write_icon_set(&spec));
        }
    }
    let _ = color_scale;
    Ok(())
}

fn write_rule(
    rule: &mut Element,
    type_: CfType,
    operator: Option<CfOperator>,
    formulas: Option<&[String]>,
    dxf_id: Option<usize>,
    color_scale: Option<&ColorScaleSpec>,
    data_bar: Option<&DataBarSpec>,
    icon_set: Option<&IconSetSpec>,
    priority: i32,
    stop_if_true: bool,
) {
    rule.set_attr("type", type_.as_str());
    rule.set_attr("priority", priority.to_string());
    if stop_if_true {
        rule.set_attr("stopIfTrue", "1");
    }
    if let Some(id) = dxf_id {
        rule.set_attr("dxfId", id.to_string());
    }
    if let Some(op) = operator {
        rule.set_attr("operator", op.as_str());
    }
    if let Some(fs) = formulas {
        for f in fs {
            rule.push_element(Element::new("formula").with_text(f.clone()));
        }
    }
    match type_ {
        CfType::ColorScale => {
            if let Some(spec) = color_scale {
                rule.push_element(write_color_scale(spec));
            }
        }
        CfType::DataBar => {
            rule.push_element(write_data_bar(data_bar.unwrap_or(&DataBarSpec {
                cfvo: None,
                color: None,
                min_length: None,
                max_length: None,
                show_value: None,
            })));
        }
        CfType::IconSet => {
            rule.push_element(write_icon_set(icon_set.unwrap_or(&IconSetSpec {
                set: None,
                cfvo: None,
                show_value: None,
                reverse: None,
                percent: None,
            })));
        }
        CfType::CellIs | CfType::Expression => {}
    }
}

fn write_color_scale(spec: &ColorScaleSpec) -> Element {
    let mut el = Element::new("colorScale");
    let n = spec.colors.len();
    let cfvo = spec.cfvo.clone().unwrap_or_else(|| default_color_cfvo(n));
    for c in &cfvo {
        el.push_element(write_cfvo(c));
    }
    for c in &spec.colors {
        el.push_element(style::color_el("color", c));
    }
    el
}

fn write_data_bar(spec: &DataBarSpec) -> Element {
    let mut el = Element::new("dataBar");
    let cfvo = spec.cfvo.clone().unwrap_or_else(|| {
        vec![
            CfvoSpec {
                type_: "min".into(),
                val: None,
                gte: None,
            },
            CfvoSpec {
                type_: "max".into(),
                val: None,
                gte: None,
            },
        ]
    });
    for c in &cfvo {
        el.push_element(write_cfvo(c));
    }
    let color = spec.color.clone().unwrap_or(ColorSpec {
        rgb: Some("FF638EC6".into()),
        theme: None,
        indexed: None,
        tint: None,
    });
    el.push_element(style::color_el("color", &color));
    if let Some(n) = spec.min_length {
        el.set_attr("minLength", n.to_string());
    }
    if let Some(n) = spec.max_length {
        el.set_attr("maxLength", n.to_string());
    }
    if spec.show_value == Some(false) {
        el.set_attr("showValue", "0");
    }
    el
}

fn write_icon_set(spec: &IconSetSpec) -> Element {
    let mut el = Element::new("iconSet");
    let set = spec.set.as_deref().unwrap_or("3TrafficLights1");
    if set != "3TrafficLights1" {
        el.set_attr("iconSet", set);
    }
    let n = icon_set_count(set);
    let cfvo = spec.cfvo.clone().unwrap_or_else(|| default_icon_cfvo(n));
    for c in &cfvo {
        el.push_element(write_cfvo(c));
    }
    if spec.show_value == Some(false) {
        el.set_attr("showValue", "0");
    }
    if spec.reverse == Some(true) {
        el.set_attr("reverse", "1");
    }
    if spec.percent == Some(false) {
        el.set_attr("percent", "0");
    }
    el
}

fn write_cfvo(spec: &CfvoSpec) -> Element {
    let mut el = Element::new("cfvo");
    el.set_attr("type", spec.type_.clone());
    if let Some(v) = &spec.val {
        el.set_attr("val", v.clone());
    }
    if spec.gte == Some(false) {
        el.set_attr("gte", "0");
    }
    el
}

fn default_color_cfvo(n: usize) -> Vec<CfvoSpec> {
    if n >= 3 {
        vec![
            CfvoSpec {
                type_: "min".into(),
                val: None,
                gte: None,
            },
            CfvoSpec {
                type_: "percentile".into(),
                val: Some("50".into()),
                gte: None,
            },
            CfvoSpec {
                type_: "max".into(),
                val: None,
                gte: None,
            },
        ]
    } else {
        vec![
            CfvoSpec {
                type_: "min".into(),
                val: None,
                gte: None,
            },
            CfvoSpec {
                type_: "max".into(),
                val: None,
                gte: None,
            },
        ]
    }
}

fn default_icon_cfvo(n: usize) -> Vec<CfvoSpec> {
    let vals: &[u32] = match n {
        3 => &[0, 33, 67],
        4 => &[0, 25, 50, 75],
        5 => &[0, 20, 40, 60, 80],
        _ => &[0, 33, 67],
    };
    vals.iter()
        .map(|v| CfvoSpec {
            type_: "percent".into(),
            val: Some(v.to_string()),
            gte: None,
        })
        .collect()
}

fn existing_formulas(rule: &Element) -> Vec<String> {
    rule.children_by_local("formula")
        .into_iter()
        .map(|f| f.text_content())
        .collect()
}

fn existing_color_scale(_rule: &Element) -> Option<ColorScaleSpec> {
    // 仅用于 check：有子元素即当作已有；具体内容不再解析。
    _rule.child_by_local("colorScale").map(|_| ColorScaleSpec {
        cfvo: None,
        colors: vec![
            ColorSpec {
                rgb: Some("FF000000".into()),
                theme: None,
                indexed: None,
                tint: None,
            },
            ColorSpec {
                rgb: Some("FFFFFFFF".into()),
                theme: None,
                indexed: None,
                tint: None,
            },
        ],
    })
}

fn existing_data_bar(rule: &Element) -> Option<DataBarSpec> {
    rule.child_by_local("dataBar").map(|_| DataBarSpec {
        cfvo: None,
        color: None,
        min_length: None,
        max_length: None,
        show_value: None,
    })
}

fn existing_icon_set(rule: &Element) -> Option<IconSetSpec> {
    rule.child_by_local("iconSet").map(|_| IconSetSpec {
        set: None,
        cfvo: None,
        show_value: None,
        reverse: None,
        percent: None,
    })
}

fn clear_typed_children(rule: &mut Element) {
    rule.children.retain(|n| match n {
        Node::Element(e) => {
            let n = e.local_name();
            n != "formula" && n != "colorScale" && n != "dataBar" && n != "iconSet"
        }
        _ => true,
    });
}

fn remove_children(el: &mut Element, local: &str) {
    el.children.retain(|n| match n {
        Node::Element(e) => e.local_name() != local,
        Node::Text(_) => true,
    });
}

fn next_priority(root: &Element) -> i32 {
    let mut max = 0i32;
    for block in root.children_by_local("conditionalFormatting") {
        for rule in block.children_by_local("cfRule") {
            if let Some(p) = rule.attr("priority").and_then(|s| s.parse::<i32>().ok()) {
                max = max.max(p);
            }
        }
    }
    max + 1
}

fn find_rule(root: &Element, index: usize) -> Option<(usize, usize)> {
    let mut n = 0usize;
    for (ci, node) in root.children.iter().enumerate() {
        let Node::Element(cf) = node else { continue };
        if cf.local_name() != "conditionalFormatting" {
            continue;
        }
        for (ri, rn) in cf.children.iter().enumerate() {
            let Node::Element(r) = rn else { continue };
            if r.local_name() != "cfRule" {
                continue;
            }
            n += 1;
            if n == index {
                return Some((ci, ri));
            }
        }
    }
    None
}

fn format_sqref(ranges: &[(CellRef, CellRef)]) -> String {
    ranges
        .iter()
        .map(|(a, b)| a1::format_range(*a, *b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn attr_true(el: &Element, name: &str) -> bool {
    matches!(
        el.attr(name),
        Some("1") | Some("true") | Some("TRUE") | Some("on")
    )
}

fn not_found(sheet: &str, index: usize, i: usize) -> Error {
    Error::with_path(
        ErrorKind::ConditionalFormatNotFound {
            id: format!("sheet:{sheet}/cf:{index}"),
        },
        format!("ops[{i}].id"),
    )
}
