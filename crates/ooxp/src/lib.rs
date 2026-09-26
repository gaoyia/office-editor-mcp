//! ooxp：用领域 JSON 对 OOXML 做 inspect / create / apply / exportMedia。

mod dispatch;
mod docx;
mod error;
mod json;
mod opc;
mod pptx;
mod xlsx;
mod xml;

pub use dispatch::run;
pub use error::{io_path, peek_lang, Error, ErrorCode, Lang};
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

    fn part_text(path: &std::path::Path, name: &str) -> String {
        let pkg = crate::opc::Package::open_bytes(fs::read(path).unwrap()).unwrap();
        String::from_utf8(pkg.get(name).unwrap().to_vec()).unwrap()
    }

    fn part_bytes(path: &std::path::Path, name: &str) -> Vec<u8> {
        let pkg = crate::opc::Package::open_bytes(fs::read(path).unwrap()).unwrap();
        pkg.get(name).unwrap().to_vec()
    }

    fn object_keys(v: &serde_json::Value) -> Vec<&str> {
        v.as_object().unwrap().keys().map(String::as_str).collect()
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
        assert_eq!(tree["pictureCount"], 0);
        assert_eq!(tree["textBoxCount"], 0);
        assert_eq!(tree["footnoteCount"], 0);
        assert_eq!(tree["endnoteCount"], 0);
        assert!(tree.get("paragraphs").is_none());
        assert!(tree.get("footnotes").is_none());
        assert_eq!(tree["headers"], json!([]));
        assert_eq!(tree["footers"], json!([]));
        assert_eq!(tree["sections"][0]["id"], "section:1");
        assert_eq!(tree["sections"][0]["headerDistance"], 708);
        assert_eq!(tree["sections"][0]["footerDistance"], 708);
        assert_eq!(tree["sections"][0]["headerLink"]["default"], false);
        assert!(tree["sections"][0]["headerLink"]["even"].is_null());
        assert!(tree["sections"][0]["headerLink"]["first"].is_null());
        assert_eq!(tree["sections"][0]["footerLink"]["default"], false);
        assert!(tree["sections"][0]["footerLink"]["even"].is_null());
        assert!(tree["sections"][0]["footerLink"]["first"].is_null());
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
        assert_eq!(
            part_bytes(&path, "/word/settings.xml"),
            part_bytes(&patched, "/word/settings.xml")
        );
    }

    #[test]
    fn docx_create_writes_compatibility_mode_15() {
        let path = tmp("settings.docx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": path.to_string_lossy(),
                "document": { "paragraphs": [{ "text": "a" }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let settings = part_text(&path, "/word/settings.xml");
        assert!(settings.contains(r#"w:name="compatibilityMode""#));
        assert!(settings.contains(r#"w:uri="http://schemas.microsoft.com/office/word""#));
        assert!(settings.contains(r#"w:val="15""#));
        let types = part_text(&path, "/[Content_Types].xml");
        assert!(types.contains("/word/settings.xml"));
        let rels = part_text(&path, "/word/_rels/document.xml.rels");
        assert!(rels.contains("/settings"));
    }

    #[test]
    fn xlsx_create_rejects_pictures_field() {
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": tmp("pic-reject.xlsx").to_string_lossy(),
                "workbook": { "sheets": [{ "name": "Sheet1" }], "pictures": [] }
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
        assert_eq!(err.field_path(), Some("workbook.pictures"));
    }

    #[test]
    fn pptx_create_rejects_pictures_field() {
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": tmp("pic-reject.pptx").to_string_lossy(),
                "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }], "pictures": [] }
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
        assert_eq!(err.field_path(), Some("presentation.pictures"));
    }

    #[test]
    fn xlsx_create_styles_use_rgb_not_theme() {
        let path = tmp("styles-rgb.xlsx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": path.to_string_lossy(),
                "workbook": { "sheets": [{ "name": "Sheet1" }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let styles = part_text(&path, "/xl/styles.xml");
        assert!(styles.contains(r#"<color rgb="FF000000"/>"#));
        assert!(!styles.contains("theme="));
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
    fn xlsx_rejects_pictures_scope_and_ops() {
        let path = tmp("pic-reject.xlsx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": path.to_string_lossy(),
                "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "inspect",
                "path": path.to_string_lossy(),
                "scope": { "pictures": true }
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "inspect",
                "path": path.to_string_lossy(),
                "scope": { "notes": true }
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "apply",
                "path": path.to_string_lossy(),
                "ops": [{ "op": "replacePicture", "id": "header:default:1/pic:1", "file": "a.png" }]
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "apply",
                "path": path.to_string_lossy(),
                "ops": [{ "op": "insertFootnote", "id": "para:1", "text": "x" }]
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
    }

    #[test]
    fn xlsx_and_pptx_reject_export_media() {
        let xlsx = tmp("export-reject.xlsx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": xlsx.to_string_lossy(),
                "workbook": { "sheets": [{ "name": "Sheet1" }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "exportMedia",
                "path": xlsx.to_string_lossy(),
                "id": "header:default:1/pic:1",
                "output": tmp("export-reject.png").to_string_lossy()
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);

        let pptx = tmp("export-reject.pptx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": pptx.to_string_lossy(),
                "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "exportMedia",
                "path": pptx.to_string_lossy(),
                "id": "slide:1/shape:2",
                "output": tmp("export-reject-ppt.png").to_string_lossy()
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidField);
    }

    #[test]
    fn pptx_create_inspect_apply_roundtrip() {
        let path = tmp("deck.pptx");
        let req = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": {
                "slides": [
                    { "shapes": [{ "text": "标题" }, { "text": "hello" }] },
                    { "shapes": [{ "text": "第二页" }] }
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
        assert_eq!(tree["id"], "pptx:presentation");
        assert_eq!(tree["slideCount"], 2);
        assert!(tree["slides"][0].get("shapes").is_none());

        let scoped = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": path.to_string_lossy(),
            "scope": { "slide": 1, "maxShapes": 200 }
        });
        let RunOutput::Inspect(tree) = run(&scoped.to_string(), None).unwrap() else {
            panic!();
        };
        let shapes = tree["slides"][0]["shapes"].as_array().unwrap();
        assert_eq!(shapes.len(), 2);
        assert_eq!(shapes[0]["id"], "slide:1/shape:2");
        assert_eq!(shapes[0]["stable"], true);
        assert_eq!(shapes[0]["text"], "标题");
        assert_eq!(shapes[1]["text"], "hello");
        assert!(tree["slides"][1].get("shapes").is_none());

        let patched = tmp("deck.patched.pptx");
        let apply = json!({
            "schemaVersion": 1,
            "action": "apply",
            "path": path.to_string_lossy(),
            "output": patched.to_string_lossy(),
            "ops": [{ "op": "setShapeText", "id": "slide:1/shape:2", "text": "新标题" }]
        });
        run(&apply.to_string(), None).unwrap();

        let again = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": patched.to_string_lossy(),
            "scope": { "slide": 1, "maxShapes": 10 }
        });
        let RunOutput::Inspect(tree) = run(&again.to_string(), None).unwrap() else {
            panic!();
        };
        assert_eq!(tree["slides"][0]["shapes"][0]["text"], "新标题");
        assert_eq!(tree["slides"][0]["shapes"][1]["text"], "hello");

        let page2 = json!({
            "schemaVersion": 1,
            "action": "inspect",
            "path": patched.to_string_lossy(),
            "scope": { "slide": 2 }
        });
        let RunOutput::Inspect(tree) = run(&page2.to_string(), None).unwrap() else {
            panic!();
        };
        assert_eq!(tree["slides"][1]["shapes"][0]["text"], "第二页");
    }

    #[test]
    fn pptx_rejects_set_cell() {
        let path = tmp("reject.pptx");
        let req = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": path.to_string_lossy(),
            "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
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
    fn pptx_empty_slides_still_one_page() {
        let path = tmp("empty.pptx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": path.to_string_lossy(),
                "presentation": { "slides": [] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let RunOutput::Inspect(tree) = run(
            &json!({
                "schemaVersion": 1,
                "action": "inspect",
                "path": path.to_string_lossy(),
                "scope": { "slide": 1 }
            })
            .to_string(),
            None,
        )
        .unwrap() else {
            panic!();
        };
        assert_eq!(tree["slideCount"], 1);
        assert_eq!(tree["slides"][0]["shapes"][0]["text"], "");
    }

    #[test]
    fn pptx_newline_roundtrip() {
        let path = tmp("nl.pptx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": path.to_string_lossy(),
                "presentation": { "slides": [{ "shapes": [{ "text": "上\n下" }] }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let RunOutput::Inspect(tree) = run(
            &json!({
                "schemaVersion": 1,
                "action": "inspect",
                "path": path.to_string_lossy(),
                "scope": { "slide": 1 }
            })
            .to_string(),
            None,
        )
        .unwrap() else {
            panic!();
        };
        assert_eq!(tree["slides"][0]["shapes"][0]["text"], "上\n下");
    }

    #[test]
    fn pptx_scope_slide_out_of_range_has_no_shapes() {
        let path = tmp("oor.pptx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": path.to_string_lossy(),
                "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let RunOutput::Inspect(tree) = run(
            &json!({
                "schemaVersion": 1,
                "action": "inspect",
                "path": path.to_string_lossy(),
                "scope": { "slide": 9 }
            })
            .to_string(),
            None,
        )
        .unwrap() else {
            panic!();
        };
        assert!(tree["slides"][0].get("shapes").is_none());
    }

    #[test]
    fn pptx_apply_slide_id_is_invalid() {
        let path = tmp("badid.pptx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": path.to_string_lossy(),
                "presentation": { "slides": [{ "shapes": [{ "text": "a" }] }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let err = run(
            &json!({
                "schemaVersion": 1,
                "action": "apply",
                "path": path.to_string_lossy(),
                "ops": [{ "op": "setShapeText", "id": "slide:1", "text": "x" }]
            })
            .to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidId);
    }

    #[test]
    fn error_message_defaults_to_english() {
        let err = run(
            r#"{"schemaVersion":2,"action":"inspect","path":"x.xlsx"}"#,
            None,
        )
        .unwrap_err();
        let en = err.to_json(Lang::En);
        assert_eq!(en["error"]["code"], "UnsupportedSchema");
        assert_eq!(en["error"]["message"], "schemaVersion must be 1");
        let zh = err.to_json(Lang::ZhCn);
        assert_eq!(zh["error"]["message"], "schemaVersion 必须是 1");
    }

    #[test]
    fn locale_is_accepted_on_inspect() {
        let err = run(
            r#"{"schemaVersion":1,"action":"inspect","path":"no-such-ooxp.xlsx","locale":"zh-CN"}"#,
            None,
        )
        .unwrap_err();
        assert_eq!(err.code(), ErrorCode::Io);
        let msg = err.to_json(Lang::En)["error"]["message"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(msg.contains("Cannot access"));
        assert!(!msg.contains("系统找不到"));
    }

    #[test]
    fn inspect_field_order_matches_contract() {
        let xlsx = tmp("order.xlsx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": xlsx.to_string_lossy(),
                "workbook": { "sheets": [{ "name": "Sheet1", "cells": [{ "ref": "A1", "value": 1 }] }] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let RunOutput::Inspect(tree) = run(
            &json!({
                "schemaVersion": 1,
                "action": "inspect",
                "path": xlsx.to_string_lossy(),
                "scope": { "sheet": "Sheet1", "range": "A1" }
            })
            .to_string(),
            None,
        )
        .unwrap() else {
            panic!();
        };
        assert_eq!(
            object_keys(&tree),
            ["schemaVersion", "format", "id", "sheets"]
        );
        assert_eq!(
            object_keys(&tree["sheets"][0]),
            ["id", "name", "kind", "truncated", "range", "cells"]
        );
        assert_eq!(
            object_keys(&tree["sheets"][0]["cells"][0]),
            ["id", "ref", "type", "value", "formula"]
        );

        let docx = tmp("order.docx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": docx.to_string_lossy(),
                "document": { "paragraphs": [
                    { "style": "Heading1", "text": "标题" },
                    { "text": "hello" }
                ] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let RunOutput::Inspect(tree) = run(
            &json!({
                "schemaVersion": 1,
                "action": "inspect",
                "path": docx.to_string_lossy(),
                "scope": { "from": 1, "maxParas": 10 }
            })
            .to_string(),
            None,
        )
        .unwrap() else {
            panic!();
        };
        assert_eq!(
            object_keys(&tree),
            [
                "schemaVersion",
                "format",
                "id",
                "paraCount",
                "pictureCount",
                "textBoxCount",
                "footnoteCount",
                "endnoteCount",
                "outline",
                "headers",
                "footers",
                "sections",
                "from",
                "truncated",
                "paragraphs"
            ]
        );
        assert_eq!(
            object_keys(&tree["outline"][0]),
            ["id", "index", "level", "style", "text"]
        );
        assert_eq!(
            object_keys(&tree["paragraphs"][0]),
            [
                "id",
                "index",
                "style",
                "level",
                "text",
                "stable",
                "pictureCount",
                "textBoxCount"
            ]
        );
        assert_eq!(
            object_keys(&tree["paragraphs"][1]),
            [
                "id",
                "index",
                "style",
                "text",
                "stable",
                "pictureCount",
                "textBoxCount"
            ]
        );
        assert_eq!(
            object_keys(&tree["sections"][0]),
            [
                "id",
                "index",
                "headerDistance",
                "footerDistance",
                "headerLink",
                "footerLink"
            ]
        );
        assert_eq!(
            object_keys(&tree["sections"][0]["headerLink"]),
            ["default", "even", "first"]
        );
        assert_eq!(
            object_keys(&tree["sections"][0]["footerLink"]),
            ["default", "even", "first"]
        );

        let pptx = tmp("order.pptx");
        run(
            &json!({
                "schemaVersion": 1,
                "action": "create",
                "path": pptx.to_string_lossy(),
                "presentation": { "slides": [
                    { "shapes": [{ "text": "标题" }] },
                    { "shapes": [{ "text": "第二页" }] }
                ] }
            })
            .to_string(),
            None,
        )
        .unwrap();
        let RunOutput::Inspect(tree) = run(
            &json!({
                "schemaVersion": 1,
                "action": "inspect",
                "path": pptx.to_string_lossy(),
                "scope": { "slide": 1 }
            })
            .to_string(),
            None,
        )
        .unwrap() else {
            panic!();
        };
        assert_eq!(
            object_keys(&tree),
            ["schemaVersion", "format", "id", "slideCount", "slides"]
        );
        assert_eq!(
            object_keys(&tree["slides"][0]),
            ["id", "index", "name", "hidden", "truncated", "shapes"]
        );
        assert_eq!(
            object_keys(&tree["slides"][1]),
            ["id", "index", "name", "hidden"]
        );
        assert_eq!(
            object_keys(&tree["slides"][0]["shapes"][0]),
            ["id", "shapeId", "name", "text", "stable"]
        );
    }
}
