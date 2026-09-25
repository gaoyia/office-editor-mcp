//! 从工作簿读出带 id 的精简树。

use serde_json::{json, Value};

use crate::error::{Error, ErrorCode};
use crate::json::InspectScope;
use crate::opc::{self, Package};
use crate::xml::{self, Element};

use super::a1;
use super::{REL_CHARTSHEET, REL_DIALOGSHEET, REL_SHARED_STRINGS};

struct SheetMeta {
    name: String,
    kind: &'static str,
    part: String,
}

pub fn inspect(bytes: &[u8], scope: Option<InspectScope>) -> Result<Value, Error> {
    let pkg = Package::open_bytes(bytes.to_vec())?;
    let wb_part = opc::office_document_part(&pkg)?;
    let wb = xml::parse(pkg.get(&wb_part)?)?;
    let rels = opc::Relationships::from_package(&pkg, &wb_part)?;

    let sheets = list_sheets(&wb.root, &rels, &wb_part, &pkg)?;
    let sst = load_shared_strings(&pkg, &wb_part, &rels)?;

    let mut sheet_json = Vec::new();
    for sh in &sheets {
        let mut obj = json!({
            "id": format!("sheet:{}", sh.name),
            "name": sh.name,
            "kind": sh.kind,
        });
        if let Some(scope) = &scope {
            if sh.name.eq_ignore_ascii_case(&scope.sheet) {
                if sh.kind != "worksheet" {
                    return Err(Error::with_path(
                        ErrorCode::InvalidField,
                        "只能对工作表读单元格",
                        "scope.sheet",
                    ));
                }
                let (cells, truncated, range) =
                    inspect_cells(&pkg, &sh.part, &sh.name, scope, &sst)?;
                obj["truncated"] = json!(truncated);
                obj["range"] = json!(range);
                obj["cells"] = Value::Array(cells);
            }
        }
        sheet_json.push(obj);
    }

    if let Some(scope) = &scope {
        if !sheets
            .iter()
            .any(|s| s.name.eq_ignore_ascii_case(&scope.sheet))
        {
            return Err(Error::with_path(
                ErrorCode::IdNotFound,
                format!("找不到工作表：{}", scope.sheet),
                "scope.sheet",
            ));
        }
    }

    Ok(json!({
        "schemaVersion": 1,
        "format": "xlsx",
        "id": "xlsx:workbook",
        "sheets": sheet_json,
    }))
}

fn list_sheets(
    workbook: &Element,
    rels: &opc::Relationships,
    wb_part: &str,
    pkg: &Package,
) -> Result<Vec<SheetMeta>, Error> {
    let sheets_el = workbook
        .child_by_local("sheets")
        .ok_or_else(|| Error::new(ErrorCode::XmlBroken, "workbook.xml 缺少 sheets"))?;
    let mut out = Vec::new();
    for sh in sheets_el.children_by_local("sheet") {
        let name = sh.attr("name").unwrap_or("").to_string();
        let rid = sh
            .attr("id")
            .or_else(|| sh.attr("r:id"))
            .ok_or_else(|| Error::new(ErrorCode::XmlBroken, "sheet 缺少 r:id"))?
            .to_string();
        let rel = rels.find_by_id(&rid).ok_or_else(|| {
            Error::new(
                ErrorCode::OpcPartMissing,
                format!("workbook 关系里没有 {rid}"),
            )
        })?;
        let kind = if rel.rel_type == REL_CHARTSHEET || rel.rel_type.ends_with("/chartsheet") {
            "chartsheet"
        } else if rel.rel_type == REL_DIALOGSHEET || rel.rel_type.ends_with("/dialogsheet") {
            "dialogsheet"
        } else {
            "worksheet"
        };
        let part = opc::resolve_rel_target(wb_part, &rel.target)?;
        if !pkg.contains(&part) {
            return Err(Error::new(
                ErrorCode::OpcPartMissing,
                format!("找不到工作表部件 {part}"),
            ));
        }
        out.push(SheetMeta { name, kind, part });
    }
    Ok(out)
}

fn load_shared_strings(
    pkg: &Package,
    wb_part: &str,
    rels: &opc::Relationships,
) -> Result<Vec<String>, Error> {
    let Some(rel) = rels
        .find_by_type_suffix("/sharedStrings")
        .into_iter()
        .next()
        .or_else(|| rels.rels.iter().find(|r| r.rel_type == REL_SHARED_STRINGS))
    else {
        return Ok(Vec::new());
    };
    if rel.is_external() {
        return Ok(Vec::new());
    }
    let part = opc::resolve_rel_target(wb_part, &rel.target)?;
    let Ok(bytes) = pkg.get(&part) else {
        return Ok(Vec::new());
    };
    let doc = xml::parse(bytes)?;
    let mut out = Vec::new();
    for si in doc.root.children_by_local("si") {
        out.push(si.text_content());
    }
    Ok(out)
}

fn inspect_cells(
    pkg: &Package,
    part: &str,
    sheet_name: &str,
    scope: &InspectScope,
    sst: &[String],
) -> Result<(Vec<Value>, bool, String), Error> {
    let (start, end) = a1::parse_range(&scope.range)?;
    let range_norm = format!(
        "{}:{}",
        a1::format_cell(start.col, start.row),
        a1::format_cell(end.col, end.row)
    );
    let doc = xml::parse(pkg.get(part)?)?;
    let Some(sheet_data) = doc.root.child_by_local("sheetData") else {
        return Ok((Vec::new(), false, range_norm));
    };

    let mut cells = Vec::new();
    let mut truncated = false;
    'rows: for row in sheet_data.children_by_local("row") {
        for c in row.children_by_local("c") {
            let Some(r) = c.attr("r") else {
                continue;
            };
            let Ok(pref) = a1::parse_cell(r) else {
                continue;
            };
            if !a1::in_range(pref, start, end) {
                continue;
            }
            if cells.len() as u32 >= scope.max_cells {
                truncated = true;
                break 'rows;
            }
            cells.push(cell_to_json(c, sheet_name, sst)?);
        }
    }
    Ok((cells, truncated, range_norm))
}

fn cell_to_json(c: &Element, sheet_name: &str, sst: &[String]) -> Result<Value, Error> {
    let raw_ref = c.attr("r").unwrap_or("");
    let pref = a1::parse_cell(raw_ref)?;
    let refer = a1::format_cell(pref.col, pref.row);
    let t = c.attr("t").unwrap_or("");
    let f_el = c.child_by_local("f");
    let v_el = c.child_by_local("v");
    let is_el = c.child_by_local("is");

    let mut formula = json!(null);
    let mut shared = false;
    if let Some(f) = f_el {
        let ft = f.attr("t").unwrap_or("");
        let text = f.text_content();
        if ft == "shared" && text.trim().is_empty() {
            formula = json!(null);
            shared = true;
        } else if !text.trim().is_empty() {
            let body = text.trim();
            let shown = if body.starts_with('=') {
                body.to_string()
            } else {
                format!("={body}")
            };
            formula = json!(shown);
            if ft == "shared" {
                shared = true;
            }
        }
    }

    let (ty, value) = classify(t, v_el, is_el, f_el.is_some(), sst);

    let id = format!("cell:{sheet_name}!{refer}");
    let mut obj = json!({
        "id": id,
        "ref": refer,
        "type": ty,
        "value": value,
        "formula": formula,
    });
    if shared && formula.is_null() {
        obj["shared"] = json!(true);
    }
    Ok(obj)
}

fn classify(
    t: &str,
    v_el: Option<&Element>,
    is_el: Option<&Element>,
    has_f: bool,
    sst: &[String],
) -> (&'static str, Value) {
    match t {
        "b" => {
            let raw = v_el.map(|e| e.text_content()).unwrap_or_default();
            let b = raw == "1" || raw.eq_ignore_ascii_case("true");
            ("boolean", json!(b))
        }
        "s" => {
            let raw = v_el.map(|e| e.text_content()).unwrap_or_default();
            let idx: usize = raw.trim().parse().unwrap_or(usize::MAX);
            let text = sst.get(idx).cloned().unwrap_or_default();
            ("text", json!(text))
        }
        "inlineStr" => {
            let text = is_el.map(|e| e.text_content()).unwrap_or_default();
            ("text", json!(text))
        }
        "str" => {
            let text = v_el.map(|e| e.text_content()).unwrap_or_default();
            ("text", json!(text))
        }
        "e" => {
            let text = v_el.map(|e| e.text_content()).unwrap_or_default();
            ("error", json!(text))
        }
        "d" => {
            let text = v_el.map(|e| e.text_content()).unwrap_or_default();
            ("date", json!(text))
        }
        "n" | "" => {
            // 省略 t 却给了 <is> 的写法少见但合法，按文本读，别当成没有值的数字。
            if let (None, Some(is)) = (v_el, is_el) {
                return ("text", json!(is.text_content()));
            }
            if v_el.is_none() && is_el.is_none() && !has_f {
                return ("empty", json!(null));
            }
            if v_el.is_none() {
                return ("number", json!(null));
            }
            let raw = v_el.unwrap().text_content();
            if let Ok(n) = raw.trim().parse::<i64>() {
                return ("number", json!(n));
            }
            if let Ok(n) = raw.trim().parse::<f64>() {
                return ("number", json!(n));
            }
            ("text", json!(raw))
        }
        _ => {
            let raw = v_el.or(is_el).map(|e| e.text_content()).unwrap_or_default();
            ("text", json!(raw))
        }
    }
}
