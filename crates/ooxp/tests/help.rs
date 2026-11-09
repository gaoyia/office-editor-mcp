use ooxp::{run, ErrorCode, RunOutput};

fn help(body: serde_json::Value) -> Result<serde_json::Value, ooxp::Error> {
    let mut request = serde_json::json!({"schemaVersion": 1, "action": "help"});
    request
        .as_object_mut()
        .unwrap()
        .extend(body.as_object().cloned().unwrap_or_default());
    match run(&request.to_string()) {
        Ok(RunOutput::Help(value)) => Ok(value),
        Ok(other) => panic!("help 应返回帮助 JSON，得到 {other:?}"),
        Err(err) => Err(err),
    }
}

#[test]
fn index_says_there_is_no_preview() {
    let value = help(serde_json::json!({})).unwrap();
    let help_action = value["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "help")
        .unwrap();
    let limits = help_action["limits"].as_str().unwrap();
    assert!(limits.contains("不支持预览"));
    assert!(limits.contains("ExportAsFixedFormat"));
    assert!(limits.contains("kwps"));
    assert!(limits.contains("不调用它们"));
    let english = help(serde_json::json!({"locale": "en"})).unwrap();
    let help_action = english["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["action"] == "help")
        .unwrap();
    let limits = help_action["limits"].as_str().unwrap();
    assert!(limits.to_lowercase().contains("does not preview"));
    assert!(limits.contains("ExportAsFixedFormat"));
    assert!(limits.contains("kwpp"));
    let found = help(serde_json::json!({"q": "预览"})).unwrap();
    assert!(found["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["action"] == "help"));
}

#[test]
fn index_lists_all_seven_actions() {
    let value = help(serde_json::json!({})).unwrap();
    let actions = value["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 7);
    let names: Vec<&str> = actions
        .iter()
        .map(|item| item["action"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "inspect",
            "create",
            "apply",
            "exportMedia",
            "exportChartData",
            "layoutCheck",
            "help",
        ]
    );
    let layout_check = actions
        .iter()
        .find(|item| item["action"] == "layoutCheck")
        .unwrap();
    assert!(!layout_check["description"].as_str().unwrap().is_empty());
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
    assert!(inspect["limits"]
        .as_str()
        .unwrap()
        .contains("不要一次要整份文件"));
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
    assert!(!set_cell["description"]
        .as_str()
        .unwrap()
        .contains("不能同时"));
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
    assert!(value.get("descriptionEn").is_none());
    assert!(value.get("limitsEn").is_none());
    let english =
        help(serde_json::json!({"format": "xlsx", "op": "setCell", "locale": "en"})).unwrap();
    assert!(english["description"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("cell"));
    assert!(english["limits"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("shared"));
    assert!(english.get("descriptionEn").is_none());
    assert!(!english["description"].as_str().unwrap().contains("单元格"));
    assert!(value["fields"]["properties"]["formula"]["pattern"]
        .as_str()
        .unwrap()
        .starts_with("^="));
    assert!(value
        .get("otherFormats")
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
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

const DATA_FROM_HELP: &str = "dataFrom 是单个单元格地址，例如 A1。不要写范围（A1:E19），不要写表名（Sheet!A1）。它不是「图表去引用哪一块已有单元格」。series 里的分类和数值会按这个格子当左上角，写进工作表，挡住的格子会被盖掉。分类写在这一列的下一行起；每个系列的名字写在这一行、向右逐列；数值写在名字下面。气泡图每个系列占两列。已经手写了表格时，把 dataFrom 指到空区域，不要指到那张表的表头。";

#[test]
fn data_from_help_sits_on_the_field() {
    let created = help(serde_json::json!({"name": "xlsxChartCreate"})).unwrap();
    assert_eq!(
        created["fields"]["properties"]["dataFrom"]["description"],
        DATA_FROM_HELP
    );
    let chart = help(serde_json::json!({"op": "insertChart"})).unwrap();
    assert_eq!(
        chart["fields"]["properties"]["dataFrom"]["description"],
        DATA_FROM_HELP
    );
    let sheet = help(serde_json::json!({"format": "xlsx", "op": "insertChartSheet"})).unwrap();
    assert_eq!(
        sheet["fields"]["properties"]["dataFrom"]["description"],
        DATA_FROM_HELP
    );
}

#[test]
fn scope_without_format_is_a_standalone_choose_format() {
    let value = help(serde_json::json!({"name": "inspectScope"})).unwrap();
    assert_eq!(value["kind"], "chooseFormat");
    assert_eq!(value["name"], "inspectScope");
    let formats: Vec<&str> = value["formats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect();
    assert_eq!(formats, ["xlsx", "docx", "pptx"]);
    assert!(value.get("fields").is_none());
    assert!(value.get("description").is_none());
}

#[test]
fn scope_with_format_keeps_only_that_formats_keys() {
    let xlsx = help(serde_json::json!({"name": "inspectScope", "format": "xlsx"})).unwrap();
    assert_eq!(xlsx["kind"], "def");
    let props = xlsx["fields"]["properties"].as_object().unwrap();
    assert!(props.contains_key("sheet"));
    assert!(!props.contains_key("from"));
    assert!(!props.contains_key("timing"));
    for (_, schema) in props {
        let formats = schema["formats"].as_array().unwrap();
        assert!(
            formats.iter().any(|item| item == "xlsx"),
            "xlsx 过滤后仍留着别的格式键: {schema:?}"
        );
    }

    let pptx = help(serde_json::json!({"name": "inspectScope", "format": "pptx"})).unwrap();
    let props = pptx["fields"]["properties"].as_object().unwrap();
    assert!(props.contains_key("timing"));
    assert!(!props.contains_key("sheet"));
}

#[test]
fn xlsm_is_folded_to_xlsx_before_scope_filter() {
    let xlsm = help(serde_json::json!({"name": "inspectScope", "format": "xlsm"})).unwrap();
    let xlsx = help(serde_json::json!({"name": "inspectScope", "format": "xlsx"})).unwrap();
    assert_eq!(xlsm["kind"], "def");
    assert_eq!(xlsm["fields"]["properties"], xlsx["fields"]["properties"]);
    assert!(xlsm["fields"]["properties"].get("sheet").is_some());
}

#[test]
fn search_reaches_defs_not_just_ops() {
    let document = help(serde_json::json!({"q": "document", "format": "docx"})).unwrap();
    let matches = document["matches"].as_array().unwrap();
    assert_eq!(matches[0]["name"], "docxDocument");
    assert_eq!(matches[0]["next"], "name=docxDocument");
    assert_ne!(
        matches[0].get("op"),
        Some(&serde_json::json!("deleteComment"))
    );
    assert!(matches.iter().any(|item| item["name"] == "docxDocument"));
    assert_eq!(
        matches
            .iter()
            .find(|item| item["name"] == "docxDocument")
            .unwrap()["open"],
        "name"
    );

    let runs = help(serde_json::json!({"q": "runs", "format": "docx"})).unwrap();
    let runs_m = runs["matches"].as_array().unwrap();
    assert_eq!(runs_m[0]["op"], "setParaRuns");
    assert_eq!(runs_m[0]["next"], "format=docx, op=setParaRuns");
    assert!(runs_m.iter().all(|item| item["name"] != "setParaRunsOp"));
    assert!(runs["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["name"] == "docxParaCreate" && item["next"] == "name=docxParaCreate"));

    let color = help(serde_json::json!({"q": "color", "format": "docx"})).unwrap();
    let color_m = color["matches"].as_array().unwrap();
    assert!(color_m.iter().any(|item| {
        item["op"] == "setParaRuns" && item["next"] == "format=docx, op=setParaRuns"
    }));
    let underline = help(serde_json::json!({"q": "underline", "format": "docx"})).unwrap();
    assert!(underline["matches"].as_array().unwrap().iter().any(|item| {
        item["op"] == "setParaRuns" && item["next"] == "format=docx, op=setParaRuns"
    }));

    let detail = help(serde_json::json!({"format": "docx", "op": "setParaRuns"})).unwrap();
    let branches = detail["fields"]["properties"]["runs"]["items"]["oneOf"]
        .as_array()
        .unwrap();
    for branch in branches.iter().skip(1) {
        let props = branch["properties"].as_object().unwrap();
        assert_eq!(props["sz"]["type"], "integer");
        assert!(props.get("font").is_some());
        assert!(props.get("highlight").is_some());
        assert!(props.get("scale").is_some());
    }

    let create = help(serde_json::json!({"q": "new file"})).unwrap();
    assert!(create["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["name"] == "createRequest" && item["kind"] == "action"));
}

#[test]
fn ask_entries_are_complete_requests() {
    let value = help(serde_json::json!({})).unwrap();
    let ask = value["ask"].as_array().unwrap();
    assert!(value.get("def").is_none());
    assert!(!ask.is_empty());
    for item in ask {
        assert_eq!(item["action"], "help");
        assert!(item.get("def").is_none());
        let again = help(item.clone()).unwrap();
        assert!(again.get("kind").is_some());
    }
}

#[test]
fn help_expands_one_ref_layer() {
    let book = help(serde_json::json!({"name": "xlsxWorkbook"})).unwrap();
    let sheet = &book["fields"]["properties"]["sheets"]["items"];
    assert!(sheet["properties"].get("name").is_some(), "{sheet}");
    assert_eq!(
        sheet["properties"]["cells"]["items"]["$ref"],
        "xlsxCellCreate"
    );

    let sheet_def = help(serde_json::json!({"name": "xlsxSheetCreate"})).unwrap();
    let cell = &sheet_def["fields"]["properties"]["cells"]["items"];
    assert_eq!(cell["properties"]["font"]["$ref"], "setCellStyleOp");

    let cell_def = help(serde_json::json!({"name": "xlsxCellCreate"})).unwrap();
    let color = &cell_def["fields"]["properties"]["font"]["properties"]["color"];
    assert_eq!(color["$ref"], "cellColor");
}

#[test]
fn cell_create_style_reuses_set_cell_style_fields() {
    let value = help(serde_json::json!({"name": "xlsxCellCreate"})).unwrap();
    let font = &value["fields"]["properties"]["font"];
    assert!(font["properties"].get("color").is_some(), "{font}");
    let fill = &value["fields"]["properties"]["fill"];
    assert!(fill["properties"].get("fg").is_some(), "{fill}");
    let fg = help(serde_json::json!({"q": "fg"})).unwrap();
    assert!(!fg["matches"].as_array().unwrap().is_empty());
    let miss = help(serde_json::json!({"q": "fgColor"})).unwrap();
    assert!(miss["matches"].as_array().unwrap().is_empty());
}

#[test]
fn shape_create_enums_exclude_connectors() {
    let value = help(serde_json::json!({"name": "pptxShapeCreate"})).unwrap();
    let align = value["fields"]["properties"]["align"]["enum"]
        .as_array()
        .unwrap();
    assert!(align.iter().any(|item| item == "l"));
    assert!(align.iter().any(|item| item.is_null()));
    let prst = value["fields"]["properties"]["prst"]["enum"]
        .as_array()
        .unwrap();
    assert!(prst.iter().any(|item| item == "ellipse"));
    assert!(prst.iter().all(|item| item != "straightConnector1"));
}

#[test]
fn unknown_op_and_mixed_keys_fail() {
    let err = help(serde_json::json!({"op": "notAnOp"})).unwrap_err();
    assert_eq!(err.code(), ErrorCode::UnsupportedOp);
    let err = help(serde_json::json!({"op": "setCell", "name": "inspectScope"})).unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidField);
}
