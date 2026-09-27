//! 工作簿定义名：inspect 名单，apply 只改 `workbook.xml` 的 `<definedNames>`。
//! 公式当字符串，不解析求值。

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind};
use crate::opc::Package;
use crate::xml::{Document, Element, Node};

use super::a1;
use super::request::{DeleteDefinedNameOp, InsertDefinedNameOp, SetDefinedNameOp};
use super::table;

/// `CT_Workbook` 子元素顺序（§A.2）。未知元素插在 `extLst` 之前。
const WORKBOOK_CHILD_ORDER: &[&str] = &[
    "fileVersion",
    "fileSharing",
    "workbookPr",
    "workbookProtection",
    "bookViews",
    "sheets",
    "functionGroups",
    "externalReferences",
    "definedNames",
    "calcPr",
    "oleSize",
    "customWorkbookViews",
    "pivotCaches",
    "smartTagPr",
    "smartTagTypes",
    "webPublishing",
    "fileRecoveryPr",
    "webPublishObjects",
    "extLst",
];

pub(crate) fn count(wb: &Element) -> usize {
    wb.child_by_local("definedNames")
        .map(|p| p.children_by_local("definedName").len())
        .unwrap_or(0)
}

pub(crate) fn inspect(wb: &Element, sheet_names: &[String]) -> Value {
    let Some(parent) = wb.child_by_local("definedNames") else {
        return json!([]);
    };
    let items: Vec<Value> = parent
        .children_by_local("definedName")
        .into_iter()
        .enumerate()
        .map(|(i, el)| inspect_one(el, i + 1, sheet_names))
        .collect();
    Value::Array(items)
}

fn inspect_one(el: &Element, n: usize, sheet_names: &[String]) -> Value {
    let mut obj = Map::new();
    obj.insert("id".into(), json!(format!("name:{n}")));
    obj.insert("name".into(), json!(el.attr("name").unwrap_or("")));
    obj.insert("formula".into(), json!(el.text_content()));
    obj.insert("hidden".into(), json!(attr_on(el, "hidden")));
    obj.insert(
        "localSheet".into(),
        json!(local_sheet_name(el, sheet_names)),
    );
    Value::Object(obj)
}

fn local_sheet_name(el: &Element, sheet_names: &[String]) -> Option<String> {
    let raw = el.attr("localSheetId")?;
    let idx: usize = raw.parse().ok()?;
    sheet_names.get(idx).cloned()
}

fn attr_on(el: &Element, name: &str) -> bool {
    matches!(
        el.attr(name).map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

pub(crate) fn apply_insert(
    pkg: &Package,
    wb_doc: &mut Document,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &[(String, Document)],
    op: &InsertDefinedNameOp,
    i: usize,
) -> Result<(), Error> {
    let scope = match op.local_sheet.as_deref() {
        Some(name) => Some(sheet_id_of(sheet_index, name, i)?),
        None => None,
    };
    check_unique(
        pkg,
        wb_doc,
        sheet_index,
        dirty_sheets,
        &op.name,
        scope,
        None,
        i,
    )?;
    let parent = ensure_defined_names(&mut wb_doc.root);
    parent.push_element(build_defined_name(&op.name, &op.formula, op.hidden, scope));
    sync_count(parent);
    Ok(())
}

pub(crate) fn apply_set(
    pkg: &Package,
    wb_doc: &mut Document,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &[(String, Document)],
    op: &SetDefinedNameOp,
    i: usize,
) -> Result<(), Error> {
    let (old_name, old_scope) = {
        let parent = wb_doc.root.child_by_local("definedNames").ok_or_else(|| {
            Error::with_path(
                ErrorKind::DefinedNameNotFound {
                    id: format!("name:{}", op.index),
                },
                format!("ops[{i}].id"),
            )
        })?;
        let current = defined_name_at(parent, op.index).ok_or_else(|| {
            Error::with_path(
                ErrorKind::DefinedNameNotFound {
                    id: format!("name:{}", op.index),
                },
                format!("ops[{i}].id"),
            )
        })?;
        (
            current.attr("name").unwrap_or("").to_string(),
            parse_local_sheet_id(current),
        )
    };
    let new_name = op.name.clone().unwrap_or(old_name);
    let new_scope = match &op.local_sheet {
        None => old_scope,
        Some(None) => None,
        Some(Some(name)) => Some(sheet_id_of(sheet_index, name, i)?),
    };
    if op.name.is_some() || op.local_sheet.is_some() {
        check_unique(
            pkg,
            wb_doc,
            sheet_index,
            dirty_sheets,
            &new_name,
            new_scope,
            Some(op.index),
            i,
        )?;
    }
    let el = defined_name_at_mut(&mut wb_doc.root, op.index).ok_or_else(|| {
        Error::with_path(
            ErrorKind::DefinedNameNotFound {
                id: format!("name:{}", op.index),
            },
            format!("ops[{i}].id"),
        )
    })?;
    if let Some(name) = &op.name {
        el.set_attr("name", name.clone());
    }
    if let Some(formula) = &op.formula {
        el.set_text(formula.clone());
    }
    if let Some(hidden) = op.hidden {
        if hidden {
            el.set_attr("hidden", "1");
        } else {
            el.remove_attr("hidden");
        }
    }
    match &op.local_sheet {
        None => {}
        Some(None) => el.remove_attr("localSheetId"),
        Some(Some(_)) => el.set_attr("localSheetId", new_scope.unwrap().to_string()),
    }
    Ok(())
}

pub(crate) fn apply_delete(
    wb_doc: &mut Document,
    op: &DeleteDefinedNameOp,
    i: usize,
) -> Result<(), Error> {
    let left = {
        let Some(parent) = wb_doc.root.child_by_local_mut("definedNames") else {
            return Err(Error::with_path(
                ErrorKind::DefinedNameNotFound {
                    id: format!("name:{}", op.index),
                },
                format!("ops[{i}].id"),
            ));
        };
        let mut seen = 0usize;
        let before = parent.children_by_local("definedName").len();
        parent.children.retain(|n| match n {
            Node::Element(e) if e.local_name() == "definedName" => {
                seen += 1;
                seen != op.index
            }
            _ => true,
        });
        let left = parent.children_by_local("definedName").len();
        if seen < op.index || before == left {
            return Err(Error::with_path(
                ErrorKind::DefinedNameNotFound {
                    id: format!("name:{}", op.index),
                },
                format!("ops[{i}].id"),
            ));
        }
        left
    };
    if left == 0 {
        wb_doc.root.children.retain(|n| match n {
            Node::Element(e) => e.local_name() != "definedNames",
            _ => true,
        });
    } else if let Some(parent) = wb_doc.root.child_by_local_mut("definedNames") {
        sync_count(parent);
    }
    Ok(())
}

pub(crate) fn validate_name(name: &str, op_index: usize) -> Result<(), Error> {
    let chars: Vec<char> = name.chars().collect();
    if chars.is_empty() || chars.len() > 255 {
        return Err(bad_name(op_index));
    }
    let first = chars[0];
    if first != '_' && first != '\\' && !first.is_alphabetic() {
        return Err(bad_name(op_index));
    }
    if chars[1..]
        .iter()
        .any(|c| *c != '_' && *c != '.' && !c.is_alphanumeric())
    {
        return Err(bad_name(op_index));
    }
    if name.eq_ignore_ascii_case("R") || name.eq_ignore_ascii_case("C") {
        return Err(bad_name(op_index));
    }
    if a1::parse_cell(name).is_ok() {
        return Err(bad_name(op_index));
    }
    if name.contains(':') && a1::parse_range(name).is_ok() {
        return Err(bad_name(op_index));
    }
    if looks_like_r1c1(name) {
        return Err(bad_name(op_index));
    }
    Ok(())
}

pub(crate) fn validate_formula(formula: &str, op_index: usize) -> Result<(), Error> {
    let t = formula.trim();
    if t.is_empty() || t.starts_with('=') || formula.starts_with('=') {
        return Err(Error::with_path(
            ErrorKind::InvalidDefinedName {
                field: "formula".into(),
            },
            format!("ops[{op_index}].formula"),
        ));
    }
    Ok(())
}

fn bad_name(op_index: usize) -> Error {
    Error::with_path(
        ErrorKind::InvalidDefinedName {
            field: "name".into(),
        },
        format!("ops[{op_index}].name"),
    )
}

fn looks_like_r1c1(name: &str) -> bool {
    let b = name.as_bytes();
    let mut i = 0;
    if i >= b.len() || !matches!(b[i], b'R' | b'r') {
        return false;
    }
    i += 1;
    i = match skip_r1c1_num(b, i) {
        Some(n) => n,
        None => return false,
    };
    if i >= b.len() || !matches!(b[i], b'C' | b'c') {
        return false;
    }
    i += 1;
    i = match skip_r1c1_num(b, i) {
        Some(n) => n,
        None => return false,
    };
    i == b.len()
}

fn skip_r1c1_num(b: &[u8], mut i: usize) -> Option<usize> {
    if i < b.len() && b[i] == b'[' {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if start == i || i >= b.len() || b[i] != b']' {
            return None;
        }
        return Some(i + 1);
    }
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    Some(i)
}

#[allow(clippy::too_many_arguments)]
fn check_unique(
    pkg: &Package,
    wb_doc: &Document,
    sheet_index: &[(String, String, &'static str)],
    dirty_sheets: &[(String, Document)],
    name: &str,
    scope: Option<u32>,
    skip: Option<usize>,
    i: usize,
) -> Result<(), Error> {
    if let Some(parent) = wb_doc.root.child_by_local("definedNames") {
        for (k, el) in parent
            .children_by_local("definedName")
            .into_iter()
            .enumerate()
        {
            if skip == Some(k + 1) {
                continue;
            }
            let Some(existing) = el.attr("name") else {
                continue;
            };
            if !existing.eq_ignore_ascii_case(name) {
                continue;
            }
            if parse_local_sheet_id(el) == scope {
                return Err(Error::with_path(
                    ErrorKind::InvalidDefinedName {
                        field: "name".into(),
                    },
                    format!("ops[{i}].name"),
                ));
            }
        }
    }
    for taken in table::table_taken_names(pkg, sheet_index, dirty_sheets)? {
        if taken.eq_ignore_ascii_case(name) {
            return Err(Error::with_path(
                ErrorKind::InvalidDefinedName {
                    field: "name".into(),
                },
                format!("ops[{i}].name"),
            ));
        }
    }
    Ok(())
}

fn sheet_id_of(
    sheet_index: &[(String, String, &'static str)],
    name: &str,
    i: usize,
) -> Result<u32, Error> {
    sheet_index
        .iter()
        .position(|(n, _, _)| n.eq_ignore_ascii_case(name))
        .map(|idx| idx as u32)
        .ok_or_else(|| {
            Error::with_path(
                ErrorKind::SheetNotFound {
                    name: name.to_string(),
                },
                format!("ops[{i}].localSheet"),
            )
        })
}

fn parse_local_sheet_id(el: &Element) -> Option<u32> {
    el.attr("localSheetId")?.parse().ok()
}

fn defined_name_at(parent: &Element, index: usize) -> Option<&Element> {
    parent
        .children_by_local("definedName")
        .into_iter()
        .nth(index - 1)
}

fn defined_name_at_mut(wb: &mut Element, index: usize) -> Option<&mut Element> {
    let parent = wb.child_by_local_mut("definedNames")?;
    let mut seen = 0usize;
    for n in parent.children.iter_mut() {
        let Node::Element(e) = n else { continue };
        if e.local_name() != "definedName" {
            continue;
        }
        seen += 1;
        if seen == index {
            return Some(e);
        }
    }
    None
}

fn ensure_defined_names(root: &mut Element) -> &mut Element {
    if root.child_by_local("definedNames").is_none() {
        insert_workbook_child(root, Element::new("definedNames"), "definedNames");
    }
    root.child_by_local_mut("definedNames").unwrap()
}

fn build_defined_name(
    name: &str,
    formula: &str,
    hidden: Option<bool>,
    scope: Option<u32>,
) -> Element {
    let mut el = Element::new("definedName").with_attr("name", name);
    if hidden == Some(true) {
        el = el.with_attr("hidden", "1");
    }
    if let Some(id) = scope {
        el = el.with_attr("localSheetId", id.to_string());
    }
    el.with_text(formula)
}

fn sync_count(parent: &mut Element) {
    let n = parent.children_by_local("definedName").len();
    parent.set_attr("count", n.to_string());
}

fn insert_workbook_child(root: &mut Element, el: Element, local: &str) {
    let want = WORKBOOK_CHILD_ORDER
        .iter()
        .position(|n| *n == local)
        .unwrap_or(WORKBOOK_CHILD_ORDER.len());
    let mut at = root.children.len();
    for (i, n) in root.children.iter().enumerate() {
        let Node::Element(e) = n else { continue };
        let pos = WORKBOOK_CHILD_ORDER
            .iter()
            .position(|n| *n == e.local_name())
            .unwrap_or(WORKBOOK_CHILD_ORDER.len());
        if pos > want {
            at = i;
            break;
        }
    }
    root.children.insert(at, Node::Element(el));
}
