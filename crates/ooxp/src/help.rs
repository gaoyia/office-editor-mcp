//! `help`：从拼好的 schema 里查动作和操作，一次只打开一层。

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde_json::{json, Map, Value};

use crate::error::{Error, ErrorKind, Lang, TypeName};
use crate::json::{self, Format};

const SCHEMA_TEXT: &str = include_str!("../schema/request.schema.json");
const SEARCH_LIMIT: usize = 20;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HelpLang {
    Zh,
    En,
}

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
    let lang = help_lang(obj)?;
    if let Some(op) = op {
        return op_detail(op, format, lang);
    }
    if let Some(name) = name {
        return def_detail(name, format, lang);
    }
    if let Some(q) = q {
        return Ok(search(q, format, lang));
    }
    if let Some(format) = format {
        return Ok(list(format, lang));
    }
    Ok(index(lang))
}

fn help_lang(obj: &Map<String, Value>) -> Result<HelpLang, Error> {
    match obj.get("locale") {
        None => Ok(HelpLang::Zh),
        Some(Value::String(tag)) => Ok(match Lang::parse(tag) {
            Lang::ZhCn => HelpLang::Zh,
            Lang::En => HelpLang::En,
        }),
        Some(_) => Err(Error::with_path(
            ErrorKind::WrongType {
                field: "locale".into(),
                expected: TypeName::String,
            },
            "locale",
        )),
    }
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

fn lookup_def_pointer(reference: &str) -> Option<Value> {
    let rest = reference.strip_prefix("#/$defs/")?;
    let (name, tail) = rest.split_once('/')?;
    let mut current = catalog().defs.get(name)?.clone();
    for part in tail.split('/') {
        let key = part.replace("~1", "/").replace("~0", "~");
        current = current.get(&key)?.clone();
    }
    Some(current)
}

fn ref_def_name(reference: &str) -> String {
    let rest = reference
        .rsplit_once("#/$defs/")
        .map(|(_, name)| name)
        .unwrap_or(reference);
    rest.split('/').next().unwrap_or(rest).to_string()
}

fn ask_query(lang: HelpLang) -> &'static str {
    match lang {
        HelpLang::Zh => "单元格",
        HelpLang::En => "cell",
    }
}

fn index(lang: HelpLang) -> Value {
    json!({
        "kind": "actions",
        "actions": [
            action_row("inspect", "inspectRequest", lang),
            action_row("create", "createRequest", lang),
            action_row("apply", "applyRequest", lang),
            action_row("exportMedia", "exportMediaRequest", lang),
            action_row("exportChartData", "exportChartDataRequest", lang),
            action_row("help", "helpRequest", lang),
        ],
        "ask": {
            "list": {"format": "xlsx"},
            "search": {"q": ask_query(lang), "format": "xlsx"},
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

fn action_row(action: &str, def_name: &str, lang: HelpLang) -> Value {
    let empty = Value::Null;
    let body = catalog().defs.get(def_name).unwrap_or(&empty);
    let description_zh = text_of(body, "description");
    let description_en = text_of(body, "descriptionEn");
    let limits_zh = text_of(body, "limits");
    let limits_en = text_of(body, "limitsEn");
    let (description, limits) = pick(
        lang,
        &description_zh,
        &description_en,
        &limits_zh,
        &limits_en,
    );
    blurb("action", action, description, limits)
}

fn list(format: &str, lang: HelpLang) -> Value {
    let mut ops = Vec::new();
    for entry in &catalog().ops {
        if entry.formats.iter().any(|item| item == format) {
            ops.push(entry_blurb("op", &entry.op, entry, lang));
        }
    }
    json!({
        "kind": "list",
        "format": format,
        "ops": ops,
    })
}

const ACTIONS: &[(&str, &str)] = &[
    ("inspect", "inspectRequest"),
    ("create", "createRequest"),
    ("apply", "applyRequest"),
    ("exportMedia", "exportMediaRequest"),
    ("exportChartData", "exportChartDataRequest"),
    ("layoutCheck", "layoutCheckRequest"),
    ("help", "helpRequest"),
];

const SKIP_DEFS: &[&str] = &["anyApplyOp", "xlsxApplyOp", "docxApplyOp", "pptxApplyOp"];

fn search(q: &str, format: Option<&str>, lang: HelpLang) -> Value {
    let needle = q.to_lowercase();
    let cat = catalog();
    let mut hits: Vec<(u8, String, Value)> = Vec::new();

    for (action, def_name) in ACTIONS {
        let body = cat.defs.get(*def_name);
        let rank = best_rank(&[*action, *def_name], body, needle.as_str());
        if let Some(rank) = rank {
            hits.push((
                rank,
                (*def_name).to_string(),
                action_hit(action, def_name, lang),
            ));
        }
    }

    for (name, body) in &cat.defs {
        if SKIP_DEFS.contains(&name.as_str()) || ACTIONS.iter().any(|(_, def)| *def == name) {
            continue;
        }
        if !def_visible(name, format) || is_op_def(cat, name) {
            continue;
        }
        let rank = best_rank(&[name.as_str()], Some(body), needle.as_str());
        if let Some(rank) = rank {
            hits.push((rank, name.clone(), def_hit(name, body, lang)));
        }
    }

    for entry in &cat.ops {
        if format.is_some_and(|item| !entry.formats.iter().any(|have| have == item)) {
            continue;
        }
        let rank = best_rank(
            &[entry.op.as_str(), entry.def_name.as_str()],
            Some(&entry.body),
            needle.as_str(),
        );
        if let Some(rank) = rank {
            hits.push((rank, entry.op.clone(), entry_blurb_formats(entry, lang)));
        }
    }

    hits.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| {
                if a.0 >= 2 {
                    hit_kind_order(&a.2).cmp(&hit_kind_order(&b.2))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then(a.1.to_lowercase().cmp(&b.1.to_lowercase()))
    });
    let total = hits.len();
    let matches: Vec<Value> = hits
        .into_iter()
        .take(SEARCH_LIMIT)
        .map(|(_, _, v)| v)
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

fn best_rank(names: &[&str], body: Option<&Value>, needle: &str) -> Option<u8> {
    let mut rank = None::<u8>;
    for name in names {
        if let Some(got) = name_rank(name, needle) {
            rank = Some(rank.map(|have| have.min(got)).unwrap_or(got));
        }
    }
    if let Some(body) = body {
        if field_hit(body, needle) {
            rank = Some(rank.map(|have| have.min(2)).unwrap_or(2));
        }
        if text_hit(body, needle, false) {
            rank = Some(rank.map(|have| have.min(3)).unwrap_or(3));
        }
        if text_hit(body, needle, true) {
            rank = Some(rank.map(|have| have.min(4)).unwrap_or(4));
        }
    }
    rank
}

fn name_rank(name: &str, needle: &str) -> Option<u8> {
    let folded = name.to_lowercase();
    if folded == needle {
        Some(0)
    } else if folded.contains(needle) {
        Some(1)
    } else {
        None
    }
}

fn hit_kind_order(value: &Value) -> u8 {
    match value.get("kind").and_then(Value::as_str) {
        Some("op") => 0,
        Some("action") => 1,
        _ => 2,
    }
}

fn is_op_def(cat: &Catalog, name: &str) -> bool {
    cat.ops.iter().any(|entry| entry.def_name == name)
}

fn field_hit(body: &Value, needle: &str) -> bool {
    property_key_hit(body, needle)
}

fn property_key_hit(value: &Value, needle: &str) -> bool {
    match value {
        Value::Array(items) => items.iter().any(|item| property_key_hit(item, needle)),
        Value::Object(obj) => {
            if let Some(props) = obj.get("properties").and_then(Value::as_object) {
                for (key, child) in props {
                    let folded = key.to_lowercase();
                    if folded == needle
                        || folded.contains(needle)
                        || property_key_hit(child, needle)
                    {
                        return true;
                    }
                }
            }
            for key in [
                "items",
                "additionalProperties",
                "unevaluatedProperties",
                "contains",
            ] {
                if let Some(child) = obj.get(key) {
                    if property_key_hit(child, needle) {
                        return true;
                    }
                }
            }
            for key in ["oneOf", "anyOf", "allOf", "prefixItems"] {
                if let Some(Value::Array(items)) = obj.get(key) {
                    if items.iter().any(|item| property_key_hit(item, needle)) {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

fn text_hit(body: &Value, needle: &str, limits: bool) -> bool {
    let keys: &[&str] = if limits {
        &["limits", "limitsEn"]
    } else {
        &["description", "descriptionEn"]
    };
    keys.iter().any(|key| {
        body.get(*key)
            .and_then(Value::as_str)
            .is_some_and(|text| text.to_lowercase().contains(needle))
    })
}

fn def_visible(name: &str, format: Option<&str>) -> bool {
    let Some(format) = format else {
        return true;
    };
    let folded = name.to_ascii_lowercase();
    for prefix in ["xlsx", "docx", "pptx"] {
        if folded.starts_with(prefix) && prefix != format {
            return false;
        }
    }
    true
}

fn action_hit(action: &str, def_name: &str, lang: HelpLang) -> Value {
    let mut row = action_row(action, def_name, lang);
    if let Some(obj) = row.as_object_mut() {
        obj.insert("kind".into(), json!("action"));
        obj.insert("name".into(), json!(def_name));
        obj.insert("open".into(), json!("name"));
        obj.insert("next".into(), json!(format!("name={def_name}")));
    }
    row
}

fn def_hit(name: &str, body: &Value, lang: HelpLang) -> Value {
    let description_zh = text_of(body, "description");
    let description_en = text_of(body, "descriptionEn");
    let limits_zh = text_of(body, "limits");
    let limits_en = text_of(body, "limitsEn");
    let (description, limits) = pick(
        lang,
        &description_zh,
        &description_en,
        &limits_zh,
        &limits_en,
    );
    let mut out = json!({
        "kind": "def",
        "name": name,
        "open": "name",
        "next": format!("name={name}"),
        "description": description,
    });
    if !limits.is_empty() {
        out["limits"] = json!(limits);
    }
    out
}

fn op_detail(op: &str, format: Option<&str>, lang: HelpLang) -> Result<Value, Error> {
    let found: Vec<&OpEntry> = catalog()
        .ops
        .iter()
        .filter(|entry| entry.op == op)
        .collect();
    if found.is_empty() {
        return Err(Error::new(ErrorKind::UnsupportedOp { op: op.into() }));
    }
    if let Some(format) = format {
        if let Some(entry) = found
            .iter()
            .find(|entry| entry.formats.iter().any(|item| item == format))
        {
            let others = other_formats(&found, entry);
            return Ok(detail_value(entry, &others, lang));
        }
    } else if found.len() == 1 {
        return Ok(detail_value(found[0], &[], lang));
    }
    let mut formats = Vec::new();
    for entry in &found {
        for item in &entry.formats {
            formats.push(entry_blurb("format", item, entry, lang));
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

fn detail_value(entry: &OpEntry, others: &[String], lang: HelpLang) -> Value {
    let (description, limits) = entry_text(entry, lang);
    let mut out = json!({
        "kind": "op",
        "op": entry.op,
        "formats": entry.formats,
        "description": description,
        "otherFormats": others,
        "fields": fields_of(&entry.body),
    });
    if !limits.is_empty() {
        out["limits"] = json!(limits);
    }
    out
}

fn def_detail(name: &str, format: Option<&str>, lang: HelpLang) -> Result<Value, Error> {
    let Some(body) = catalog().defs.get(name) else {
        return Err(Error::with_path(
            ErrorKind::UnknownField {
                field: name.to_string(),
            },
            "name",
        ));
    };
    if name == "inspectScope" && format.is_none() {
        return Ok(json!({
            "kind": "chooseFormat",
            "name": "inspectScope",
            "formats": ["xlsx", "docx", "pptx"],
        }));
    }
    let description_zh = text_of(body, "description");
    let description_en = text_of(body, "descriptionEn");
    let limits_zh = text_of(body, "limits");
    let limits_en = text_of(body, "limitsEn");
    let (description, limits) = pick(
        lang,
        &description_zh,
        &description_en,
        &limits_zh,
        &limits_en,
    );
    let mut out = json!({
        "kind": "def",
        "name": name,
        "description": description,
        "fields": fields_of(body),
    });
    if let Some(format) = format {
        if name == "inspectScope" {
            retain_scope_fields(&mut out, format);
        }
    }
    if !limits.is_empty() {
        out["limits"] = json!(limits);
    }
    Ok(out)
}

fn retain_scope_fields(out: &mut Value, format: &str) {
    let Some(props) = out
        .pointer_mut("/fields/properties")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    props.retain(|_, schema| {
        schema
            .get("formats")
            .and_then(Value::as_array)
            .is_some_and(|formats| formats.iter().any(|f| f.as_str() == Some(format)))
    });
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

fn entry_blurb(key: &str, name: &str, entry: &OpEntry, lang: HelpLang) -> Value {
    let (description, limits) = entry_text(entry, lang);
    blurb(key, name, description, limits)
}

fn entry_blurb_formats(entry: &OpEntry, lang: HelpLang) -> Value {
    let (description, limits) = entry_text(entry, lang);
    let next = if entry.formats.len() == 1 {
        format!("format={}, op={}", entry.formats[0], entry.op)
    } else {
        format!("op={}", entry.op)
    };
    let mut out = json!({
        "kind": "op",
        "op": entry.op,
        "formats": entry.formats,
        "open": "op",
        "next": next,
        "description": description,
    });
    if !limits.is_empty() {
        out["limits"] = json!(limits);
    }
    out
}

fn blurb(key: &str, name: &str, description: &str, limits: &str) -> Value {
    let mut out = Map::new();
    out.insert(key.to_string(), json!(name));
    out.insert("description".into(), json!(description));
    if !limits.is_empty() {
        out.insert("limits".into(), json!(limits));
    }
    Value::Object(out)
}

fn entry_text(entry: &OpEntry, lang: HelpLang) -> (&str, &str) {
    pick(
        lang,
        &entry.description,
        &entry.description_en,
        &entry.limits,
        &entry.limits_en,
    )
}

fn pick<'a>(
    lang: HelpLang,
    description: &'a str,
    description_en: &'a str,
    limits: &'a str,
    limits_en: &'a str,
) -> (&'a str, &'a str) {
    match lang {
        HelpLang::Zh => (description, limits),
        HelpLang::En => (
            if description_en.is_empty() {
                description
            } else {
                description_en
            },
            if limits_en.is_empty() {
                limits
            } else {
                limits_en
            },
        ),
    }
}

fn shorten_refs(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            if map.len() == 1 {
                if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                    if let Some(resolved) = lookup_def_pointer(reference) {
                        return shorten_refs(&resolved);
                    }
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
