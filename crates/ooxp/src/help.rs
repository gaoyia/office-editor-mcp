//! `help`：从拼好的 schema 里查动作和操作，一次只打开一层。

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, TypeName};
use crate::json::{self, Format};

const SCHEMA_TEXT: &str = include_str!("../../../docs/json-schema/request.schema.json");
const SEARCH_LIMIT: usize = 20;

struct OpEntry {
    op: String,
    def_name: String,
    formats: Vec<String>,
    description: String,
    description_en: String,
    limits: String,
    limits_en: String,
    body: Value,
}

struct Catalog {
    ops: Vec<OpEntry>,
    defs: Map<String, Value>,
}

pub fn help(obj: &Map<String, Value>) -> Result<Value, Error> {
    let format = optional_str(obj, "format")?;
    let q = optional_str(obj, "q")?;
    let op = optional_str(obj, "op")?;
    let name = optional_str(obj, "name")?;
    if op.is_some() && name.is_some() {
        return Err(Error::new(ErrorKind::MutuallyExclusive {
            a: "op".into(),
            b: "name".into(),
        }));
    }
    if q.is_some() && op.is_some() {
        return Err(Error::new(ErrorKind::MutuallyExclusive {
            a: "q".into(),
            b: "op".into(),
        }));
    }
    if q.is_some() && name.is_some() {
        return Err(Error::new(ErrorKind::MutuallyExclusive {
            a: "q".into(),
            b: "name".into(),
        }));
    }
    let format = match format {
        Some(value) => Some(format_key(value)?),
        None => None,
    };
    if let Some(op) = op {
        return op_detail(op, format);
    }
    if let Some(name) = name {
        return def_detail(name);
    }
    if let Some(q) = q {
        return Ok(search(q, format));
    }
    if let Some(format) = format {
        return Ok(list(format));
    }
    Ok(index())
}

fn optional_str<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>, Error> {
    match obj.get(key) {
        None => Ok(None),
        Some(Value::String(value)) if !value.is_empty() => Ok(Some(value)),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: key.to_string(),
                expected: TypeName::NonEmptyString,
            },
            key,
        )),
    }
}

fn format_key(value: &str) -> Result<&'static str, Error> {
    match json::parse_format(value)? {
        Format::Xlsx => Ok("xlsx"),
        Format::Docx => Ok("docx"),
        Format::Pptx => Ok("pptx"),
    }
}

fn catalog() -> &'static Catalog {
    static CELL: OnceLock<Catalog> = OnceLock::new();
    CELL.get_or_init(load_catalog)
}

fn load_catalog() -> Catalog {
    let document: Value = serde_json::from_str(SCHEMA_TEXT).expect("schema 不是 JSON");
    let defs = document["$defs"]
        .as_object()
        .expect("schema 缺少 $defs")
        .clone();
    let mut formats_of: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (format, catalog_name) in [
        ("xlsx", "xlsxApplyOp"),
        ("docx", "docxApplyOp"),
        ("pptx", "pptxApplyOp"),
    ] {
        let items = defs[catalog_name]["oneOf"]
            .as_array()
            .unwrap_or_else(|| panic!("{catalog_name} 缺少 oneOf"));
        for item in items {
            let def_name = ref_def_name(item["$ref"].as_str().unwrap_or_else(|| {
                panic!("{catalog_name} 有条目没有 $ref");
            }));
            formats_of
                .entry(def_name)
                .or_default()
                .push(format.to_string());
        }
    }
    let mut ops = Vec::new();
    for (def_name, formats) in formats_of {
        let body = defs
            .get(&def_name)
            .unwrap_or_else(|| panic!("名单指向了不存在的 {def_name}"))
            .clone();
        let op = body["properties"]["op"]["const"]
            .as_str()
            .unwrap_or_else(|| panic!("{def_name} 缺少 op"))
            .to_string();
        let description = text_of(&body, "description");
        let description_en = text_of(&body, "descriptionEn");
        let limits = text_of(&body, "limits");
        let limits_en = text_of(&body, "limitsEn");
        ops.push(OpEntry {
            op,
            def_name,
            formats,
            description,
            description_en,
            limits,
            limits_en,
            body,
        });
    }
    ops.sort_by(|a, b| a.op.cmp(&b.op).then(a.def_name.cmp(&b.def_name)));
    Catalog { ops, defs }
}

fn ref_def_name(reference: &str) -> String {
    let rest = reference
        .rsplit_once("#/$defs/")
        .map(|(_, name)| name)
        .unwrap_or(reference);
    rest.split('/').next().unwrap_or(rest).to_string()
}

fn index() -> Value {
    json!({
        "kind": "actions",
        "actions": [
            action_row("inspect", "inspectRequest"),
            action_row("create", "createRequest"),
            action_row("apply", "applyRequest"),
            action_row("exportMedia", "exportMediaRequest"),
            action_row("exportChartData", "exportChartDataRequest"),
            action_row("help", "helpRequest"),
        ],
        "ask": {
            "list": {"format": "xlsx"},
            "search": {"q": "单元格", "format": "xlsx"},
            "op": {"format": "xlsx", "op": "setCell"},
            "def": {"name": "inspectScope"}
        }
    })
}

fn text_of(body: &Value, key: &str) -> String {
    body.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn action_row(action: &str, def_name: &str) -> Value {
    let empty = Value::Null;
    let body = catalog().defs.get(def_name).unwrap_or(&empty);
    blurb(
        "action",
        action,
        &text_of(body, "description"),
        &text_of(body, "descriptionEn"),
        &text_of(body, "limits"),
        &text_of(body, "limitsEn"),
    )
}

fn list(format: &str) -> Value {
    let mut ops = Vec::new();
    for entry in &catalog().ops {
        if entry.formats.iter().any(|item| item == format) {
            ops.push(entry_blurb("op", &entry.op, entry));
        }
    }
    json!({
        "kind": "list",
        "format": format,
        "ops": ops,
    })
}

fn search(q: &str, format: Option<&str>) -> Value {
    let needle = q.to_lowercase();
    let mut hits: Vec<&OpEntry> = catalog()
        .ops
        .iter()
        .filter(|entry| {
            format.is_none_or(|item| entry.formats.iter().any(|have| have == item))
                && (entry.op.to_lowercase().contains(&needle)
                    || entry.description.to_lowercase().contains(&needle)
                    || entry.description_en.to_lowercase().contains(&needle)
                    || entry.limits.to_lowercase().contains(&needle)
                    || entry.limits_en.to_lowercase().contains(&needle))
        })
        .collect();
    hits.sort_by(|a, b| {
        let a_name = a.op.to_lowercase().contains(&needle);
        let b_name = b.op.to_lowercase().contains(&needle);
        b_name
            .cmp(&a_name)
            .then(a.op.cmp(&b.op))
            .then(a.def_name.cmp(&b.def_name))
    });
    let total = hits.len();
    let matches: Vec<Value> = hits
        .into_iter()
        .take(SEARCH_LIMIT)
        .map(|entry| {
            entry_blurb_formats(entry)
        })
        .collect();
    let mut out = json!({
        "kind": "search",
        "q": q,
        "total": total,
        "matches": matches,
    });
    if let Some(format) = format {
        out["format"] = json!(format);
    }
    if total > SEARCH_LIMIT {
        out["truncated"] = json!(true);
    }
    out
}

fn op_detail(op: &str, format: Option<&str>) -> Result<Value, Error> {
    let found: Vec<&OpEntry> = catalog().ops.iter().filter(|entry| entry.op == op).collect();
    if found.is_empty() {
        return Err(Error::new(ErrorKind::UnsupportedOp { op: op.into() }));
    }
    if let Some(format) = format {
        if let Some(entry) = found
            .iter()
            .find(|entry| entry.formats.iter().any(|item| item == format))
        {
            let others = other_formats(&found, entry);
            return Ok(detail_value(entry, &others));
        }
    } else if found.len() == 1 {
        return Ok(detail_value(found[0], &[]));
    }
    let mut formats = Vec::new();
    for entry in &found {
        for item in &entry.formats {
            let mut row = Map::new();
            row.insert("format".into(), json!(item));
            row.insert("description".into(), json!(entry.description));
            let mut row = Value::Object(row);
            attach_pair(&mut row, entry);
            formats.push(row);
        }
    }
    let mut out = json!({
        "kind": "chooseFormat",
        "op": op,
        "formats": formats,
    });
    if let Some(format) = format {
        out["asked"] = json!(format);
    }
    Ok(out)
}

fn other_formats(found: &[&OpEntry], picked: &OpEntry) -> Vec<String> {
    let mut others = Vec::new();
    for entry in found {
        if entry.def_name == picked.def_name {
            continue;
        }
        for format in &entry.formats {
            if !others.contains(format) {
                others.push(format.clone());
            }
        }
    }
    others
}

fn detail_value(entry: &OpEntry, others: &[String]) -> Value {
    let mut out = json!({
        "kind": "op",
        "op": entry.op,
        "formats": entry.formats,
        "description": entry.description,
        "otherFormats": others,
        "fields": fields_of(&entry.body),
    });
    attach_pair(&mut out, entry);
    out
}

fn def_detail(name: &str) -> Result<Value, Error> {
    let Some(body) = catalog().defs.get(name) else {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: name.to_string(),
            },
            "name",
        ));
    };
    let description = text_of(body, "description");
    let mut out = json!({
        "kind": "def",
        "name": name,
        "description": description,
        "fields": fields_of(body),
    });
    attach_named(
        &mut out,
        &text_of(body, "descriptionEn"),
        &text_of(body, "limits"),
        &text_of(body, "limitsEn"),
    );
    Ok(out)
}

fn fields_of(body: &Value) -> Value {
    let mut value = shorten_refs(body);
    if let Some(obj) = value.as_object_mut() {
        obj.remove("description");
        obj.remove("descriptionEn");
        obj.remove("limits");
        obj.remove("limitsEn");
    }
    value
}

fn entry_blurb(key: &str, name: &str, entry: &OpEntry) -> Value {
    blurb(
        key,
        name,
        &entry.description,
        &entry.description_en,
        &entry.limits,
        &entry.limits_en,
    )
}

fn entry_blurb_formats(entry: &OpEntry) -> Value {
    let mut out = json!({
        "op": entry.op,
        "formats": entry.formats,
        "description": entry.description,
    });
    attach_named(
        &mut out,
        &entry.description_en,
        &entry.limits,
        &entry.limits_en,
    );
    out
}

fn blurb(
    key: &str,
    name: &str,
    description: &str,
    description_en: &str,
    limits: &str,
    limits_en: &str,
) -> Value {
    let mut out = Map::new();
    out.insert(key.to_string(), json!(name));
    out.insert("description".into(), json!(description));
    let mut value = Value::Object(out);
    attach_named(&mut value, description_en, limits, limits_en);
    value
}

fn attach_pair(out: &mut Value, entry: &OpEntry) {
    attach_named(
        out,
        &entry.description_en,
        &entry.limits,
        &entry.limits_en,
    );
}

fn attach_named(out: &mut Value, description_en: &str, limits: &str, limits_en: &str) {
    if !description_en.is_empty() {
        out["descriptionEn"] = json!(description_en);
    }
    if !limits.is_empty() {
        out["limits"] = json!(limits);
    }
    if !limits_en.is_empty() {
        out["limitsEn"] = json!(limits_en);
    }
}

fn shorten_refs(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            if map.len() == 1 {
                if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                    return json!({ "$ref": ref_def_name(reference) });
                }
            }
            let mut out = Map::new();
            for (key, child) in map {
                out.insert(key.clone(), shorten_refs(child));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(shorten_refs).collect()),
        other => other.clone(),
    }
}
