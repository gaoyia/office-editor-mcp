//! 浏览器演示的门口。只调用 `run_bytes`，不读磁盘。
//! 页面负责授权目录里的字节。时间和随机数在这里装入。

use std::collections::BTreeMap;

use serde_json::{json, Value};

use ooxp::{install_clock, peek_lang, run_bytes, ByteOutput, EnvGuard, Lang};

/// 一次调用的结果。字节由页面写回授权目录。
#[derive(Debug)]
pub enum Outcome {
    Json(Value),
    Bytes {
        path: String,
        bytes: Vec<u8>,
        content_type: Option<String>,
        ops_applied: Option<usize>,
    },
    Fail(Value),
}

/// 页面侧的一次会话。附属文件按领域 JSON 里的原路径存放。
pub struct Host {
    files: BTreeMap<String, Vec<u8>>,
    _guard: EnvGuard,
}

impl Host {
    pub fn boot() -> Self {
        Self {
            files: BTreeMap::new(),
            _guard: install_clock(now_nanos, unix_secs_millis, pid, fill_random),
        }
    }

    pub fn clear_files(&mut self) {
        self.files.clear();
    }

    pub fn put_file(&mut self, name: &str, bytes: &[u8]) {
        self.files.insert(name.to_string(), bytes.to_vec());
    }

    /// `package` 是 inspect、apply、export 要打开的包。附属文件事先放进 `put_file`。
    pub fn run_inner(&self, request_json: &str, package: Option<&[u8]>) -> Outcome {
        let lang = peek_lang(request_json).unwrap_or(Lang::ZhCn);
        let parsed = serde_json::from_str::<Value>(request_json).ok();
        if let Some(value) = parsed.as_ref() {
            let action = value.get("action").and_then(Value::as_str).unwrap_or("");
            if needs_package(action) && package.is_none() {
                let path = value.get("path").and_then(Value::as_str).unwrap_or("");
                return Outcome::Fail(json!({
                    "ok": false,
                    "error": {
                        "code": "Io",
                        "message": format!("目录里没有「{path}」"),
                        "path": "path"
                    }
                }));
            }
        }
        let refs: Vec<(&str, &[u8])> = self
            .files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect();
        match run_bytes(request_json, package, &refs) {
            Ok(ByteOutput::Inspect(value) | ByteOutput::Help(value)) => Outcome::Json(value),
            Ok(ByteOutput::DryRun { ops_applied, parts }) => {
                Outcome::Json(ooxp::dry_run_value(ops_applied, &parts))
            }
            Ok(ByteOutput::Bytes {
                bytes,
                ops_applied,
                content_type,
            }) => Outcome::Bytes {
                path: parsed
                    .as_ref()
                    .map(output_name)
                    .unwrap_or_else(|| "output.bin".to_string()),
                bytes,
                content_type,
                ops_applied,
            },
            Err(err) => Outcome::Fail(err.to_json(lang)),
        }
    }
}

fn needs_package(action: &str) -> bool {
    matches!(
        action,
        "inspect"
            | "apply"
            | "exportMedia"
            | "exportChartData"
            | "exportChartImage"
            | "exportSlideImage"
            | "exportPageImage"
            | "layoutCheck"
    )
}

fn output_name(value: &Value) -> String {
    let action = value.get("action").and_then(Value::as_str).unwrap_or("");
    let pick = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
    match action {
        "apply" => pick("output").or_else(|| pick("path")),
        "exportMedia" | "exportChartData" | "exportChartImage" | "exportSlideImage"
        | "exportPageImage" => pick("output"),
        _ => pick("path"),
    }
    .unwrap_or_else(|| "output.bin".to_string())
}

/// 浏览器里没有进程号。
fn pid() -> u128 {
    1
}

#[cfg(target_arch = "wasm32")]
fn now_nanos() -> u128 {
    (js_sys::Date::now() * 1_000_000.0) as u128
}

#[cfg(not(target_arch = "wasm32"))]
fn now_nanos() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn unix_secs_millis() -> (u64, u32) {
    let ms = js_sys::Date::now();
    if !ms.is_finite() || ms < 0.0 {
        return (0, 0);
    }
    let whole = ms.floor() as u64;
    (whole / 1000, (whole % 1000) as u32)
}

#[cfg(not(target_arch = "wasm32"))]
fn unix_secs_millis() -> (u64, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (dur.as_secs(), dur.subsec_millis())
}

#[cfg(target_arch = "wasm32")]
fn fill_random(buf: &mut [u8]) {
    let window = web_sys::window().expect("需要浏览器 window");
    let crypto = window.crypto().expect("需要 Web Crypto");
    crypto
        .get_random_values_with_u8_array(buf)
        .expect("getRandomValues 失败");
}

/// 本机测试没有 Web Crypto。演示页在 wasm 上走上面的分支。
#[cfg(not(target_arch = "wasm32"))]
fn fill_random(buf: &mut [u8]) {
    let mut n = 0x9e3779b9u32;
    for byte in buf {
        n = n.wrapping_mul(1664525).wrapping_add(1013904223);
        *byte = (n >> 24) as u8;
    }
}

#[cfg(target_arch = "wasm32")]
mod page;

#[cfg(test)]
mod tests {
    use super::*;

    const CREATE: &str = r#"{
        "schemaVersion": 1,
        "action": "create",
        "path": "book.xlsx",
        "format": "xlsx",
        "workbook": {
            "sheets": [{
                "name": "Sheet1",
                "cells": [{ "ref": "A1", "value": "hi" }]
            }]
        }
    }"#;

    #[test]
    fn create_then_inspect_uses_returned_bytes() {
        let host = Host::boot();
        let made = host.run_inner(CREATE, None);
        let bytes = match made {
            Outcome::Bytes { path, bytes, .. } => {
                assert_eq!(path, "book.xlsx");
                assert!(bytes.starts_with(b"PK"));
                bytes
            }
            Outcome::Fail(value) => panic!("{value}"),
            Outcome::Json(value) => panic!("{value}"),
        };
        let seen = host.run_inner(
            r#"{"action":"inspect","path":"book.xlsx","format":"xlsx","scope":{"sheet":"Sheet1","range":"A1"}}"#,
            Some(&bytes),
        );
        match seen {
            Outcome::Json(value) => assert!(value.to_string().contains("hi")),
            Outcome::Fail(value) => panic!("{value}"),
            Outcome::Bytes { .. } => panic!("inspect 不应返回字节"),
        }
    }

    #[test]
    fn demo_showcase_writes_three_formats() {
        let host = Host::boot();
        let cases = [
            (
                include_str!("../../../demo/web/samples/xlsx.json"),
                "ooxp-example-销售经营看板.xlsx",
                r#"{"schemaVersion":1,"action":"apply","path":"ooxp-example-销售经营看板.xlsx","ops":[{"op":"setCell","id":"cell:总览!A1","value":"2026 销售经营看板 · 已改"},{"op":"setCell","id":"cell:总览!C10","value":920}]}"#,
                "ooxp-example-销售经营看板.xlsx",
            ),
            (
                include_str!("../../../demo/web/samples/docx.json"),
                "ooxp-example-经营分析报告.docx",
                r#"{"schemaVersion":1,"action":"apply","path":"ooxp-example-经营分析报告.docx","ops":[{"op":"setParaText","id":"para:00000001","text":"2026 年度产品经营分析报告 · 已改"}]}"#,
                "ooxp-example-经营分析报告.docx",
            ),
            (
                include_str!("../../../demo/web/samples/pptx.json"),
                "ooxp-example-产品战略发布.pptx",
                r#"{"schemaVersion":1,"action":"apply","path":"ooxp-example-产品战略发布.pptx","ops":[{"op":"setShapeText","id":"slide:1/shape:7","text":"2026 产品战略发布 · 已改"}]}"#,
                "ooxp-example-产品战略发布.pptx",
            ),
        ];
        for (create, created, apply, patched) in cases {
            let made = host.run_inner(create, None);
            let bytes = match made {
                Outcome::Bytes { path, bytes, .. } => {
                    assert_eq!(path, created);
                    assert!(bytes.starts_with(b"PK"));
                    bytes
                }
                Outcome::Fail(value) => panic!("{created}: {value}"),
                Outcome::Json(value) => panic!("{created}: {value}"),
            };
            match host.run_inner(apply, Some(&bytes)) {
                Outcome::Bytes { path, bytes, .. } => {
                    assert_eq!(path, patched);
                    assert!(bytes.starts_with(b"PK"));
                }
                Outcome::Fail(value) => panic!("{patched}: {value}"),
                Outcome::Json(value) => panic!("{patched}: {value}"),
            }
        }
    }

    #[test]
    fn demo_create_then_apply_for_three_formats() {
        let host = Host::boot();
        let cases = [
            (
                r#"{"schemaVersion":1,"action":"create","path":"book.xlsx","format":"xlsx","workbook":{"sheets":[{"name":"Sheet1","cells":[{"ref":"A1","value":1},{"ref":"A2","value":"hello"},{"ref":"A3","value":true},{"ref":"B1","formula":"=SUM(A1)"}]}]}}"#,
                "book.xlsx",
                r#"{"schemaVersion":1,"action":"apply","path":"book.xlsx","output":"book.patched.xlsx","ops":[{"op":"setCell","id":"cell:Sheet1!A1","value":42},{"op":"setCell","id":"cell:Sheet1!A2","value":"已修改"}]}"#,
                "book.patched.xlsx",
            ),
            (
                r#"{"schemaVersion":1,"action":"create","path":"note.docx","format":"docx","document":{"paragraphs":[{"style":"Heading1","text":"标题"},{"text":"hello"}]}}"#,
                "note.docx",
                r#"{"schemaVersion":1,"action":"apply","path":"note.docx","output":"note.patched.docx","ops":[{"op":"setParaText","id":"para:00000001","text":"新标题"}]}"#,
                "note.patched.docx",
            ),
            (
                r#"{"schemaVersion":1,"action":"create","path":"deck.pptx","format":"pptx","presentation":{"slides":[{"shapes":[{"text":"标题"},{"text":"hello"}]},{"shapes":[{"text":"第二页"}]}]}}"#,
                "deck.pptx",
                r#"{"schemaVersion":1,"action":"apply","path":"deck.pptx","output":"deck.patched.pptx","ops":[{"op":"setShapeText","id":"slide:1/shape:2","text":"新标题"}]}"#,
                "deck.patched.pptx",
            ),
        ];
        for (create, created, apply, patched) in cases {
            let made = host.run_inner(create, None);
            let bytes = match made {
                Outcome::Bytes { path, bytes, .. } => {
                    assert_eq!(path, created);
                    bytes
                }
                Outcome::Fail(value) => panic!("{created}: {value}"),
                Outcome::Json(value) => panic!("{created}: {value}"),
            };
            match host.run_inner(apply, Some(&bytes)) {
                Outcome::Bytes { path, bytes, .. } => {
                    assert_eq!(path, patched);
                    assert!(bytes.starts_with(b"PK"));
                }
                Outcome::Fail(value) => panic!("{patched}: {value}"),
                Outcome::Json(value) => panic!("{patched}: {value}"),
            }
        }
    }

    #[test]
    fn inspect_without_bytes_names_the_path() {
        let host = Host::boot();
        match host.run_inner(r#"{"action":"inspect","path":"no.xlsx"}"#, None) {
            Outcome::Fail(value) => {
                assert_eq!(value["ok"], false);
                assert!(value["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("no.xlsx"));
            }
            other => panic!("{other:?}"),
        }
    }
}
