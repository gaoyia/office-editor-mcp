use ooxp::{run, ErrorCode, RunOutput};

fn help(body: serde_json::Value) -> Result<serde_json::Value, ooxp::Error> {
    let mut request = serde_json::json!({"schemaVersion": 1, "action": "help"});
    request
        .as_object_mut()
        .unwrap()
        .extend(body.as_object().cloned().unwrap_or_default());
    match run(&request.to_string(), None) {
        Ok(RunOutput::Help(value)) => Ok(value),
        Ok(other) => panic!("help 应返回帮助 JSON，得到 {other:?}"),
        Err(err) => Err(err),
    }
}

#[test]
fn index_tells_actions_apart() {
    let value = help(serde_json::json!({})).unwrap();
    assert_eq!(value["kind"], "actions");
    let actions = value["actions"].as_array().unwrap();
    let inspect = actions
        .iter()
        .find(|item| item["action"] == "inspect")
        .unwrap();
    assert!(inspect["description"]
        .as_str()
        .unwrap()
        .contains("看文件的一部分"));
    assert!(inspect["limits"].as_str().unwrap().contains("不要一次要整份文件"));
    let apply = actions
        .iter()
        .find(|item| item["action"] == "apply")
        .unwrap();
    assert!(apply["limits"]
        .as_str()
        .unwrap()
        .contains("不要把整棵 inspect 树"));
}

#[test]
fn list_xlsx_has_set_cell_not_paragraph() {
    let value = help(serde_json::json!({"format": "xlsx"})).unwrap();
    let ops = value["ops"].as_array().unwrap();
    assert!(ops.iter().any(|item| item["op"] == "setCell"));
    assert!(ops.iter().all(|item| item["op"] != "setParaText"));
    let set_cell = ops.iter().find(|item| item["op"] == "setCell").unwrap();
    assert!(set_cell["description"]
        .as_str()
        .unwrap()
        .contains("值或公式"));
    assert!(!set_cell["description"].as_str().unwrap().contains("不能同时"));
}

#[test]
fn search_finds_shared_formula_word() {
    let value = help(serde_json::json!({"q": "共享公式", "format": "xlsx"})).unwrap();
    let matches = value["matches"].as_array().unwrap();
    assert!(matches.iter().any(|item| item["op"] == "setCell"));
    let english = help(serde_json::json!({"q": "shared formula", "format": "xlsx"})).unwrap();
    let matches = english["matches"].as_array().unwrap();
    assert!(matches.iter().any(|item| item["op"] == "setCell"));
}

#[test]
fn set_cell_fields_stay_one_layer() {
    let value = help(serde_json::json!({"format": "xlsx", "op": "setCell"})).unwrap();
    assert_eq!(value["kind"], "op");
    assert!(value["description"].as_str().unwrap().contains("值或公式"));
    assert!(value["limits"].as_str().unwrap().contains("不能同时出现"));
    assert!(value["limits"].as_str().unwrap().contains("共享公式"));
    assert!(value["descriptionEn"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("cell"));
    assert!(value["limitsEn"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("shared"));
    assert!(value["fields"]["properties"]["formula"]["pattern"]
        .as_str()
        .unwrap()
        .starts_with("^="));
    assert!(value.get("otherFormats").unwrap().as_array().unwrap().is_empty());
}

#[test]
fn insert_math_does_not_merge_formats() {
    let value = help(serde_json::json!({"op": "insertMath"})).unwrap();
    assert_eq!(value["kind"], "chooseFormat");
    let formats: Vec<_> = value["formats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["format"].as_str().unwrap())
        .collect();
    assert!(formats.contains(&"xlsx"));
    assert!(formats.contains(&"docx"));
    assert!(formats.contains(&"pptx"));
    assert!(value.get("fields").is_none());

    let xlsx = help(serde_json::json!({"format": "xlsx", "op": "insertMath"})).unwrap();
    assert!(xlsx["fields"]["properties"].get("expr").is_none());
    assert!(xlsx["fields"]["properties"].get("from").is_some());
    let others = xlsx["otherFormats"].as_array().unwrap();
    assert!(others.iter().any(|item| item == "docx"));

    let docx = help(serde_json::json!({"format": "docx", "op": "insertMath"})).unwrap();
    assert!(docx["fields"]["properties"].get("expr").is_some());
    assert!(docx["description"].as_str().unwrap().contains("插入公式"));
    assert!(docx["limits"].as_str().unwrap().contains("不要套用 xlsx"));
}

#[test]
fn scope_is_a_definition_not_an_op() {
    let value = help(serde_json::json!({"name": "inspectScope"})).unwrap();
    assert_eq!(value["kind"], "def");
    assert!(value["description"].as_str().unwrap().contains("查看范围"));
    assert!(value["limits"].as_str().unwrap().contains("不能混用"));
    assert!(value["fields"]["properties"].get("sheet").is_some());
}

#[test]
fn unknown_op_and_mixed_keys_fail() {
    let err = help(serde_json::json!({"op": "notAnOp"})).unwrap_err();
    assert_eq!(err.code(), ErrorCode::UnsupportedOp);
    let err = help(serde_json::json!({"op": "setCell", "name": "inspectScope"})).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
