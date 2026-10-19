//! 解析器认的 apply 动作必须和 Schema 里按格式分开的名单一致。

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn schema_dir() -> PathBuf {
    repo_root().join("docs/json-schema")
}

fn read_json(name: &str) -> Value {
    let text = fs::read_to_string(schema_dir().join(name)).unwrap_or_else(|err| {
        panic!("读 {name} 失败: {err}");
    });
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("{name} 不是 JSON: {err}"))
}

fn resolve<'a>(request: &'a Value, siblings: &'a [(&'a str, Value)], reference: &str) -> &'a Value {
    if let Some(name) = reference.strip_prefix("#/$defs/") {
        return &request["$defs"][name];
    }
    let (file, frag) = reference.split_once("#/$defs/").unwrap_or_else(|| {
        panic!("无法解析引用 {reference}");
    });
    let body = siblings
        .iter()
        .find(|(name, _)| *name == file)
        .unwrap_or_else(|| panic!("缺少 {file}"))
        .1
        .as_object()
        .unwrap();
    &body["$defs"][frag]
}

fn op_const(defn: &Value) -> String {
    defn["properties"]["op"]["const"]
        .as_str()
        .unwrap_or_else(|| panic!("动作定义缺少 op 常量: {defn}"))
        .to_string()
}

fn list_ops(
    request: &Value,
    siblings: &[(&str, Value)],
    key: &str,
    combiner: &str,
    unique: bool,
) -> BTreeSet<String> {
    let items = request["$defs"][key][combiner]
        .as_array()
        .unwrap_or_else(|| panic!("{key} 缺少 {combiner}"));
    let mut names = BTreeSet::new();
    for item in items {
        let reference = item["$ref"]
            .as_str()
            .unwrap_or_else(|| panic!("{key} 有条目没有 $ref"));
        let name = op_const(resolve(request, siblings, reference));
        let inserted = names.insert(name.clone());
        if unique {
            assert!(inserted, "{key} 里 {name} 出现了两次");
        }
    }
    names
}

fn parser_ops(relative: &str) -> BTreeSet<String> {
    let text = fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|err| panic!("读 {relative} 失败: {err}"));
    let start = text
        .find("pub fn parse_ops")
        .unwrap_or_else(|| panic!("{relative} 没有 parse_ops"));
    let rest = &text[start + 1..];
    let end = ["\nfn ", "\npub fn ", "\npub(super) fn "]
        .iter()
        .filter_map(|mark| rest.find(mark))
        .min()
        .unwrap_or(rest.len());
    let mut names = BTreeSet::new();
    for line in rest[..end].lines() {
        let trimmed = line.trim();
        let Some(after_quote) = trimmed.strip_prefix('"') else {
            continue;
        };
        let Some(end_quote) = after_quote.find('"') else {
            continue;
        };
        let name = &after_quote[..end_quote];
        let after = after_quote[end_quote + 1..].trim_start();
        if after.starts_with("=>") {
            names.insert(name.to_string());
        }
    }
    assert!(!names.is_empty(), "{relative} 没有解析到动作");
    names
}

#[test]
fn apply_schema_matches_parsers() {
    let request = read_json("request.schema.json");
    let siblings = [
        ("xlsx.schema.json", read_json("xlsx.schema.json")),
        ("docx.schema.json", read_json("docx.schema.json")),
        ("pptx.schema.json", read_json("pptx.schema.json")),
    ];
    let xlsx_parser = parser_ops("crates/ooxp/src/xlsx/request/mod.rs");
    let docx_parser = parser_ops("crates/ooxp/src/docx/request/mod.rs");
    let pptx_parser = parser_ops("crates/ooxp/src/pptx/request/mod.rs");
    let xlsx_schema = list_ops(&request, &siblings, "xlsxApplyOp", "oneOf", true);
    let docx_schema = list_ops(&request, &siblings, "docxApplyOp", "oneOf", true);
    let pptx_schema = list_ops(&request, &siblings, "pptxApplyOp", "oneOf", true);
    let any_schema = list_ops(&request, &siblings, "anyApplyOp", "anyOf", false);

    assert_eq!(xlsx_schema, xlsx_parser, "xlsx 动作名单不一致");
    assert_eq!(docx_schema, docx_parser, "docx 动作名单不一致");
    assert_eq!(pptx_schema, pptx_parser, "pptx 动作名单不一致");

    let mut union = xlsx_parser.clone();
    union.extend(docx_parser.iter().cloned());
    union.extend(pptx_parser.iter().cloned());
    assert_eq!(any_schema, union, "未给 format 时的动作名单应是三家并集");

    assert!(xlsx_schema.contains("setCell"));
    assert!(!xlsx_schema.contains("setParaText"));
    assert!(docx_schema.contains("setParaText"));
    assert!(!docx_schema.contains("setCell"));
    assert!(pptx_schema.contains("insertOle"));
    assert!(pptx_schema.contains("deleteOle"));
    assert!(!pptx_schema.contains("insertTextBox"));
}
