//! ooxp：用领域 JSON 对 OOXML 做 inspect / create / apply。

mod dispatch;
mod docx;
mod error;
mod json;
mod opc;
mod xlsx;
mod xml;

pub use dispatch::run;
pub use error::{Error, ErrorCode};
pub use json::{written_json, RunOutput};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("ooxp-tests");
        let _ = fs::create_dir_all(&dir);
        dir.join(name)
    }

    #[test]
    fn create_inspect_apply_roundtrip() {
        let path = tmp("book.xlsx");
        let req = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "format": "xlsx",
            "workbook": {
                "sheets": [{
                    "name": "Sheet1",
                    "cells": [
                        { "ref": "A1", "value": 1 },
                        { "ref": "A2", "value": "hello" },
                        { "ref": "A3", "value": true },
                        { "ref": "B1", "formula": "=SUM(A1)" }
                    ]
                }]
            }
        });
        let out = run(&req.to_string(), None).unwrap();
        match out {
            RunOutput::Written { .. } => {}
            _ => panic!("create 应写出文件"),
        }

        let inspect_req = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "A1:D20" }
        });
        let RunOutput::Inspect(tree) = run(&inspect_req.to_string(), None).unwrap() else {
            panic!("inspect 应返回树");
        };
        assert_eq!(tree["id"], "xlsx:workbook");
        let cells = tree["sheets"][0]["cells"].as_array().unwrap();
        assert!(cells
            .iter()
            .any(|c| c["id"] == "cell:Sheet1!A1" && c["value"] == 1));
        assert!(cells
            .iter()
            .any(|c| c["id"] == "cell:Sheet1!A2" && c["value"] == "hello"));
        assert!(cells
            .iter()
            .any(|c| c["id"] == "cell:Sheet1!B1" && c["formula"] == "=SUM(A1)"));

        let patched = tmp("book.patched.xlsx");
        let apply_req = json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": patched.to_string_lossy(),
            "ops": [
                { "op": "setCell", "id": "cell:Sheet1!A1", "value": 42 },
                { "op": "setCell", "id": "cell:Sheet1!C1", "value": null }
            ]
        });
        run(&apply_req.to_string(), None).unwrap();

        let inspect2 = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": patched.to_string_lossy(),
            "scope": { "sheet": "Sheet1", "range": "A1:C1" }
        });
        let RunOutput::Inspect(tree2) = run(&inspect2.to_string(), None).unwrap() else {
            panic!();
        };
        let cells2 = tree2["sheets"][0]["cells"].as_array().unwrap();
        assert!(cells2
            .iter()
            .any(|c| c["id"] == "cell:Sheet1!A1" && c["value"] == 42));
        assert!(!cells2.iter().any(|c| c["id"] == "cell:Sheet1!C1"));
    }

    #[test]
    fn apply_rejects_whole_tree() {
        let err = run(
            r#"{"schemaVersion":1,"action":"apply","path":"a.xlsx","sheets":[]}"#,
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
    }

    #[test]
    fn inspect_without_scope_has_no_cells() {
        let path = tmp("sheets-only.xlsx");
        let req = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "workbook": { "sheets": [{ "name": "Sheet1" }] }
        });
        run(&req.to_string(), None).unwrap();
        let inspect_req = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        });
        let RunOutput::Inspect(tree) = run(&inspect_req.to_string(), None).unwrap() else {
            panic!();
        };
        assert!(tree["sheets"][0].get("cells").is_none());
    }

    #[test]
    fn docx_create_inspect_apply_roundtrip() {
        let path = tmp("note.docx");
        let req = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": {
                "paragraphs": [
                    { "style": "Heading1", "text": "标题" },
                    { "text": "hello" }
                ]
            }
        });
        run(&req.to_string(), None).unwrap();

        let outline = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy()
        });
        let RunOutput::Inspect(tree) = run(&outline.to_string(), None).unwrap() else {
            panic!();
        };
        assert_eq!(tree["id"], "docx:document");
        assert_eq!(tree["paraCount"], 2);
        assert!(tree.get("paragraphs").is_none());
        assert_eq!(tree["outline"][0]["level"], 1);
        assert_eq!(tree["outline"][0]["text"], "标题");

        let scoped = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 200 }
        });
        let RunOutput::Inspect(tree) = run(&scoped.to_string(), None).unwrap() else {
            panic!();
        };
        let paras = tree["paragraphs"].as_array().unwrap();
        assert_eq!(paras.len(), 2);
        assert!(paras[0]["id"].as_str().unwrap().starts_with("para:"));
        assert_eq!(paras[0]["stable"], true);
        assert_eq!(paras[1]["text"], "hello");
        let heading_id = paras[0]["id"].as_str().unwrap().to_string();

        let patched = tmp("note.patched.docx");
        let apply = json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": patched.to_string_lossy(),
            "ops": [{ "op": "setParaText", "id": heading_id, "text": "新标题" }]
        });
        run(&apply.to_string(), None).unwrap();

        let again = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": patched.to_string_lossy(),
            "scope": { "from": 1, "maxParas": 10 }
        });
        let RunOutput::Inspect(tree) = run(&again.to_string(), None).unwrap() else {
            panic!();
        };
        assert_eq!(tree["paragraphs"][0]["text"], "新标题");
        assert_eq!(tree["paragraphs"][0]["style"], "Heading1");
        assert_eq!(tree["paragraphs"][1]["text"], "hello");
    }

    #[test]
    fn docx_rejects_set_cell() {
        let path = tmp("reject.docx");
        let req = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "document": { "paragraphs": [{ "text": "a" }] }
        });
        run(&req.to_string(), None).unwrap();
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "apply",
                "path": path.to_string_lossy(),
                "ops": [{ "op": "setCell", "id": "cell:Sheet1!A1", "value": 1 }]
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
    }

    #[test]
    fn pptx_still_not_implemented() {
        let err = run(
            r#"{"schemaVersion":1,"action":"inspect","path":"a.pptx"}"#,
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::NotImplemented);
    }
}
