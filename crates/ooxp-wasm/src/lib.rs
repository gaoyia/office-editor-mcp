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
        "inspect" | "apply" | "exportMedia" | "exportChartData" | "layoutCheck"
    )
}

fn output_name(value: &Value) -> String {
    let action = value.get("action").and_then(Value::as_str).unwrap_or("");
    let pick = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
    match action {
        "apply" => pick("output").or_else(|| pick("path")),
        "exportMedia" | "exportChartData" => pick("output"),
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
        let mut host = Host::boot();
        let png = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x90, 0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08,
            0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xFE,
            0x02, 0xFE, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        for name in [
            "mark.png",
            "bg-cover.png",
            "bg-cards.png",
            "bg-split.png",
            "bg-steps.png",
            "bg-chart.png",
            "bg-close.png",
            "deco-dot.png",
            "deco-ring.png",
            "deco-bar.png",
        ] {
            host.put_file(name, &png);
        }
        let xlsx = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": "ooxp-example-业绩.xlsx",
            "format": "xlsx",
            "workbook": {
                "theme": {
                    "name": "业绩",
                    "colors": {
                        "accent1": "0F6E6E",
                        "accent2": "1D4E89",
                        "accent3": "C2410C",
                        "accent4": "7C3AED"
                    },
                    "fonts": { "majorLatin": "Calibri", "majorEa": "微软雅黑" }
                },
                "sheets": [
                    {
                        "name": "业绩",
                        "cells": [
                            { "ref": "A1", "value": "本季业绩", "font": { "bold": true, "sz": 18, "color": { "rgb": "F7F4EE" } }, "fill": { "pattern": "solid", "fg": { "rgb": "0F6E6E" } }, "align": { "horizontal": "center", "vertical": "center" } },
                            { "ref": "A3", "value": "产品", "font": { "bold": true, "color": { "rgb": "FFFFFF" } }, "fill": { "pattern": "solid", "fg": { "rgb": "1C1917" } } },
                            { "ref": "B3", "value": "一月", "font": { "bold": true, "color": { "rgb": "FFFFFF" } }, "fill": { "pattern": "solid", "fg": { "rgb": "1C1917" } } },
                            { "ref": "C3", "value": "二月", "font": { "bold": true, "color": { "rgb": "FFFFFF" } }, "fill": { "pattern": "solid", "fg": { "rgb": "1C1917" } } },
                            { "ref": "D3", "value": "三月", "font": { "bold": true, "color": { "rgb": "FFFFFF" } }, "fill": { "pattern": "solid", "fg": { "rgb": "1C1917" } } },
                            { "ref": "E3", "value": "合计", "font": { "bold": true, "color": { "rgb": "FFFFFF" } }, "fill": { "pattern": "solid", "fg": { "rgb": "1C1917" } } },
                            { "ref": "F3", "value": "趋势", "font": { "bold": true, "color": { "rgb": "FFFFFF" } }, "fill": { "pattern": "solid", "fg": { "rgb": "1C1917" } } },
                            { "ref": "A4", "value": "甲" }, { "ref": "B4", "value": 42, "numFmt": "#,##0" }, { "ref": "C4", "value": 38, "numFmt": "#,##0" }, { "ref": "D4", "value": 55, "numFmt": "#,##0" }, { "ref": "E4", "formula": "=SUM(B4:D4)", "numFmt": "#,##0", "font": { "bold": true } },
                            { "ref": "A5", "value": "乙" }, { "ref": "B5", "value": 28, "numFmt": "#,##0" }, { "ref": "C5", "value": 61, "numFmt": "#,##0" }, { "ref": "D5", "value": 33, "numFmt": "#,##0" }, { "ref": "E5", "formula": "=SUM(B5:D5)", "numFmt": "#,##0", "font": { "bold": true } },
                            { "ref": "A6", "value": "丙" }, { "ref": "B6", "value": 51, "numFmt": "#,##0" }, { "ref": "C6", "value": 22, "numFmt": "#,##0" }, { "ref": "D6", "value": 47, "numFmt": "#,##0" }, { "ref": "E6", "formula": "=SUM(B6:D6)", "numFmt": "#,##0", "font": { "bold": true } },
                            { "ref": "A7", "value": "丁" }, { "ref": "B7", "value": 19, "numFmt": "#,##0" }, { "ref": "C7", "value": 44, "numFmt": "#,##0" }, { "ref": "D7", "value": 36, "numFmt": "#,##0" }, { "ref": "E7", "formula": "=SUM(B7:D7)", "numFmt": "#,##0", "font": { "bold": true } }
                        ],
                        "cols": [
                            { "col": "A", "width": 14 }, { "col": "B", "width": 12 }, { "col": "C", "width": 12 },
                            { "col": "D", "width": 12 }, { "col": "E", "width": 12 }, { "col": "F", "width": 16 },
                            { "col": "G", "width": 3 }
                        ],
                        "rows": [{ "row": 1, "height": 36 }, { "row": 3, "toRow": 7, "height": 22 }],
                        "merges": ["A1:F1"],
                        "tables": [{ "ref": "A3:E7", "name": "Quarter", "columns": ["产品", "一月", "二月", "三月", "合计"] }],
                        "charts": [
                            {
                                "type": "combo",
                                "title": "三个月",
                                "from": "H3",
                                "dataFrom": "AA3",
                                "cx": 5029200,
                                "cy": 2286000,
                                "series": [
                                    { "name": "一月", "type": "col", "categories": ["甲", "乙", "丙", "丁"], "values": [42, 28, 51, 19] },
                                    { "name": "二月", "type": "col", "categories": ["甲", "乙", "丙", "丁"], "values": [38, 61, 22, 44] },
                                    { "name": "三月", "type": "line", "categories": ["甲", "乙", "丙", "丁"], "values": [55, 33, 47, 36] }
                                ]
                            },
                            {
                                "type": "doughnut",
                                "title": "合计占比",
                                "from": "H17",
                                "dataFrom": "AA12",
                                "cx": 5029200,
                                "cy": 2286000,
                                "series": [{ "name": "合计", "categories": ["甲", "乙", "丙", "丁"], "values": [135, 122, 120, 99] }]
                            }
                        ],
                        "conditionalFormats": [
                            { "sqref": "B4:B7", "type": "dataBar", "dataBar": { "color": { "rgb": "0F6E6E" } } },
                            { "sqref": "C4:C7", "type": "colorScale", "colorScale": { "colors": [{ "rgb": "F8696B" }, { "rgb": "FFEB84" }, { "rgb": "63BE7B" }] } },
                            { "sqref": "D4:D7", "type": "iconSet" }
                        ],
                        "sparklines": [
                            { "ref": "F4", "formula": "B4:D4", "type": "column", "high": true, "low": true },
                            { "ref": "F5", "formula": "B5:D5", "type": "column", "high": true, "low": true },
                            { "ref": "F6", "formula": "B6:D6", "type": "column", "high": true, "low": true },
                            { "ref": "F7", "formula": "B7:D7", "type": "column", "high": true, "low": true }
                        ],
                        "smartArts": [{
                            "layout": "cycle",
                            "from": "A10",
                            "cx": 5029200,
                            "cy": 2286000,
                            "points": [{ "text": "创建" }, { "text": "图表" }, { "text": "条件格式" }, { "text": "改格" }]
                        }],
                        "comments": [{ "cell": "A1", "text": "旁边有组合图和环形图，下一张表是整页图。", "author": "ooxp" }],
                        "headerFooters": [
                            { "kind": "odd", "side": "header", "text": "&C本季业绩" },
                            { "kind": "odd", "side": "footer", "text": "&R第 &P 页" }
                        ],
                        "view": { "frozenRows": 3, "zoom": 110 }
                    },
                    {
                        "name": "总览",
                        "kind": "chartsheet",
                        "type": "col",
                        "title": "季度对比",
                        "dataSheet": "业绩",
                        "dataFrom": "AA21",
                        "series": [
                            { "name": "一月", "categories": ["甲", "乙", "丙", "丁"], "values": [42, 28, 51, 19] },
                            { "name": "二月", "categories": ["甲", "乙", "丙", "丁"], "values": [38, 61, 22, 44] },
                            { "name": "三月", "categories": ["甲", "乙", "丙", "丁"], "values": [55, 33, 47, 36] }
                        ]
                    }
                ]
            }
        }).to_string();
        let docx = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": "ooxp-example-发布说明.docx",
            "format": "docx",
            "document": {
                "theme": {
                    "name": "说明",
                    "colors": { "accent1": "0F6E6E", "accent2": "C2410C", "accent3": "1D4E89" },
                    "fonts": { "majorLatin": "Calibri", "minorLatin": "Cambria", "majorEa": "微软雅黑", "minorEa": "宋体" }
                },
                "pageBackground": { "color": "FBF7F1" },
                "watermarks": [{ "text": "OOXP" }],
                "pageBorders": {
                    "offsetFrom": "page",
                    "top": { "val": "single", "sz": 18, "space": 14, "color": "0F6E6E" },
                    "left": { "val": "single", "sz": 18, "space": 14, "color": "0F6E6E" },
                    "bottom": { "val": "single", "sz": 18, "space": 14, "color": "0F6E6E" },
                    "right": { "val": "single", "sz": 18, "space": 14, "color": "0F6E6E" }
                },
                "numberings": [{ "kind": "bullet" }],
                "headers": [{ "type": "default", "paragraphs": [{ "text": "ooxp  ·  发布说明" }] }],
                "footers": [{ "type": "default", "paragraphs": [{ "text": "第 ", "fields": [{ "instr": " PAGE " }] }] }],
                "paragraphs": [
                    {
                        "style": "Heading1",
                        "text": "今晚发布",
                        "footnote": "脚注、页眉和页码写在各自分部里。",
                        "endnote": "尾注写在文末。",
                        "bookmarks": [{ "name": "top" }],
                        "comments": [{ "text": "批注", "author": "ooxp", "replies": [{ "text": "回复" }] }]
                    },
                    {
                        "runs": [
                            { "text": "三种格式", "sz": 28, "bold": true, "color": "0F6E6E" },
                            { "text": "，一次写进", "sz": 28 },
                            { "text": "同一个包", "sz": 28, "bold": true, "color": "1D4E89" },
                            { "text": "。", "sz": 28 }
                        ],
                        "shd": { "fill": "E7F3F1" },
                        "hyperlinks": [{ "text": "仓库", "url": "https://github.com/gaoyia/ooxp" }]
                    },
                    { "text": "工作簿写表和图", "num": 1 },
                    { "text": "文档写版式和图示", "num": 1 },
                    { "text": "演示文稿写页和切换", "num": 1 },
                    {
                        "runs": [
                            { "text": "黑体", "font": "SimHei", "eastAsiaFont": "黑体", "sz": 32, "bold": true },
                            { "text": "  ", "sz": 32 },
                            { "text": "楷体", "font": "KaiTi", "eastAsiaFont": "楷体", "sz": 32 },
                            { "text": "  ", "sz": 32 },
                            { "text": "Cambria", "font": "Cambria", "sz": 32, "italic": true, "color": "1D4E89" },
                            { "text": "  ", "sz": 32 },
                            { "text": "等宽", "font": "Consolas", "eastAsiaFont": "Consolas", "sz": 28, "highlight": "yellow" }
                        ]
                    },
                    {
                        "runs": [
                            { "text": "窄", "scale": 50, "sz": 44, "eastAsiaFont": "微软雅黑" },
                            { "text": "正", "scale": 100, "sz": 44, "eastAsiaFont": "微软雅黑" },
                            { "text": "宽", "scale": 200, "sz": 44, "color": "C2410C", "eastAsiaFont": "微软雅黑" },
                            { "text": " 疏", "spacing": 80, "sz": 32, "underline": "wave", "color": "0F6E6E" },
                            { "text": "密", "spacing": -12, "sz": 32 },
                            { "text": "字宽", "eastAsianLayout": { "combine": true, "combineBrackets": "square" }, "sz": 32 }
                        ],
                        "borders": { "bottom": { "val": "single", "sz": 12, "color": "0F6E6E" } }
                    },
                    { "text": "", "maths": [{ "text": "a^2+b^2=c^2", "kind": "display" }] },
                    { "pictures": [{ "file": "mark.png", "cx": 2743200, "cy": 1828800 }] },
                    {
                        "text": "汉字",
                        "rubies": [{ "base": "汉字", "rt": "hànzì" }],
                        "crossRefs": [{ "bookmark": "top" }]
                    },
                    {
                        "contentControls": [
                            { "kind": "checkbox", "checked": true },
                            { "kind": "dropDown", "text": "甲", "items": ["甲", "乙", "丙"] }
                        ],
                        "fields": [{ "instr": " DATE ", "result": "2026-11-08" }]
                    },
                    {
                        "text": "循环",
                        "smartArts": [{
                            "layout": "cycle",
                            "kind": "inline",
                            "cx": 5029200,
                            "cy": 2286000,
                            "points": [{ "text": "创建" }, { "text": "打开" }, { "text": "修改" }, { "text": "保存" }]
                        }]
                    },
                    {
                        "text": "份额",
                        "charts": [{
                            "type": "doughnut",
                            "kind": "inline",
                            "cx": 4572000,
                            "cy": 2743200,
                            "title": "份额",
                            "series": [{ "name": "格式", "categories": ["工作簿", "文档", "演示文稿"], "values": [5, 3, 4] }]
                        }]
                    }
                ],
                "tables": [
                    {
                        "after": 13,
                        "align": "center",
                        "look": { "firstRow": true, "bandedRows": false, "firstColumn": false },
                        "width": { "type": "dxa", "w": 9000 },
                        "cols": [3000, 3000, 3000],
                        "rowHeights": [
                            { "h": 460, "rule": "atLeast" },
                            { "h": 720, "rule": "atLeast" }
                        ],
                        "cellMar": { "top": 80, "left": 120, "bottom": 80, "right": 120 },
                        "borders": {
                            "top": { "val": "single", "sz": 12, "color": "0F6E6E" },
                            "left": { "val": "single", "sz": 12, "color": "0F6E6E" },
                            "bottom": { "val": "single", "sz": 12, "color": "0F6E6E" },
                            "right": { "val": "single", "sz": 12, "color": "0F6E6E" },
                            "insideH": { "val": "single", "sz": 4, "color": "D6D3D1" },
                            "insideV": { "val": "single", "sz": 8, "color": "FFFFFF" }
                        },
                        "cellFormats": [
                            { "r": 1, "c": 1, "fill": "D8EFEA", "vAlign": "center" },
                            { "r": 1, "c": 2, "fill": "F4E4D7", "vAlign": "center" },
                            { "r": 1, "c": 3, "fill": "D6E4F5", "vAlign": "center" },
                            { "r": 2, "c": 1, "fill": "FBF7F1", "vAlign": "center" },
                            { "r": 2, "c": 2, "fill": "FBF7F1", "vAlign": "center" },
                            { "r": 2, "c": 3, "fill": "FBF7F1", "vAlign": "center" }
                        ],
                        "cells": [
                            ["工作簿", "文档", "演示文稿"],
                            ["表和图", "图示和公式", "形状和切换"]
                        ]
                    }
                ]
            }
        }).to_string();
        let pics = |bg: &str| {
            json!([
                { "file": bg, "x": 0, "y": 0, "cx": 12192000, "cy": 6858000 },
                { "file": "deco-dot.png", "x": 180000, "y": 140000, "cx": 380000, "cy": 380000 },
                { "file": "deco-ring.png", "x": 11620000, "y": 120000, "cx": 460000, "cy": 460000 },
                { "file": "deco-bar.png", "x": 220000, "y": 6360000, "cx": 1280000, "cy": 260000 }
            ])
        };
        let pptx = json!({
            "schemaVersion": 1,
            "action": "create",
            "path": "ooxp-example-发布会.pptx",
            "format": "pptx",
            "presentation": {
                "sections": [
                    { "name": "开场", "start": 1 },
                    { "name": "图", "start": 3 },
                    { "name": "数字", "start": 5 }
                ],
                "handout": { "header": "ooxp", "footer": "发布会" },
                "slides": [
                    {
                        "shapes": [
                            { "text": "今晚发布", "sz": 5400, "bold": true, "color": "0F6E6E", "align": "ctr" },
                            { "text": "六页排开", "sz": 2000, "color": "1C1917", "align": "ctr" }
                        ],
                        "pictures": pics("bg-cover.png"),
                        "tables": [{
                            "rows": 2,
                            "cols": 3,
                            "x": 900000,
                            "y": 2700000,
                            "cx": 10400000,
                            "cy": 3100000,
                            "cells": [
                                [
                                    { "text": "工作簿", "fill": "D8EFEA" },
                                    { "text": "文档", "fill": "F4E4D7" },
                                    { "text": "演示文稿", "fill": "D6E4F5" }
                                ],
                                [
                                    { "text": "表和图", "fill": "FBF7F1" },
                                    { "text": "图示和公式", "fill": "FBF7F1" },
                                    { "text": "形状和切换", "fill": "FBF7F1" }
                                ]
                            ]
                        }],
                        "comments": [{ "text": "封面批注", "author": "ooxp", "x": 10800000, "y": 520000 }],
                        "transition": { "type": "fade", "dur": 800 },
                        "anims": [
                            { "type": "fade", "shape": 1, "trigger": "with", "dur": 480 },
                            { "type": "fade", "picture": 2, "trigger": "with", "delay": 40, "dur": 560 },
                            { "type": "zoom", "picture": 3, "trigger": "with", "delay": 120, "dur": 680 },
                            { "type": "flyIn", "picture": 4, "trigger": "with", "delay": 140, "dur": 640 },
                            { "type": "flyIn", "shape": 2, "trigger": "with", "delay": 80, "dur": 520 },
                            { "type": "fade", "table": 1, "trigger": "with", "delay": 120, "dur": 520 },
                            { "type": "growShrink", "picture": 3, "trigger": "after", "dur": 880, "repeat": 2 }
                        ],
                        "notes": "封面。第一个文本框是 slide:1/shape:2。"
                    },
                    {
                        "shapes": [{ "text": "三件事", "sz": 3600, "bold": true, "color": "1C1917" }],
                        "pictures": pics("bg-cards.png"),
                        "tables": [{
                            "rows": 2,
                            "cols": 3,
                            "x": 900000,
                            "y": 1850000,
                            "cx": 10400000,
                            "cy": 4100000,
                            "cells": [
                                [
                                    { "text": "工作簿", "fill": "D8EFEA" },
                                    { "text": "文档", "fill": "F4E4D7" },
                                    { "text": "演示文稿", "fill": "D6E4F5" }
                                ],
                                [
                                    { "text": "表、图、条件格式", "fill": "FBF7F1" },
                                    { "text": "版式、图示、公式", "fill": "FBF7F1" },
                                    { "text": "页、形状、切换", "fill": "FBF7F1" }
                                ]
                            ]
                        }],
                        "transition": { "type": "push", "dir": "l", "dur": 700 },
                        "anims": [
                            { "type": "fade", "shape": 1, "trigger": "with", "dur": 480 },
                            { "type": "fade", "picture": 2, "trigger": "with", "delay": 40, "dur": 560 },
                            { "type": "fade", "table": 1, "trigger": "with", "delay": 100, "dur": 520 }
                        ],
                        "notes": "三张色卡。"
                    },
                    {
                        "shapes": [{ "text": "版式", "sz": 3600, "bold": true, "color": "1C1917" }],
                        "pictures": pics("bg-split.png"),
                        "smartArts": [{
                            "layout": "list",
                            "x": 1400000,
                            "y": 1850000,
                            "cx": 9400000,
                            "cy": 4100000,
                            "points": [
                                { "text": "页边" },
                                { "text": "页眉" },
                                { "text": "公式" },
                                { "text": "水印" }
                            ]
                        }],
                        "transition": { "type": "wipe", "dir": "r", "dur": 700 },
                        "anims": [
                            { "type": "fade", "shape": 1, "trigger": "with", "dur": 480 },
                            { "type": "zoom", "picture": 3, "trigger": "with", "delay": 80, "dur": 600 },
                            { "type": "fade", "smartArt": 1, "trigger": "with", "delay": 100, "dur": 600 }
                        ],
                        "notes": "左图右清单。"
                    },
                    {
                        "shapes": [{ "text": "四步", "sz": 3600, "bold": true, "color": "1C1917" }],
                        "pictures": pics("bg-steps.png"),
                        "smartArts": [{
                            "layout": "cycle",
                            "x": 1600000,
                            "y": 1850000,
                            "cx": 9000000,
                            "cy": 4100000,
                            "points": [
                                { "text": "创建" },
                                { "text": "打开" },
                                { "text": "修改" },
                                { "text": "保存" }
                            ]
                        }],
                        "transition": { "type": "wheel", "spokes": 4, "dur": 700 },
                        "anims": [
                            { "type": "fade", "picture": 2, "trigger": "with", "dur": 500 },
                            { "type": "zoom", "smartArt": 1, "trigger": "with", "delay": 80, "dur": 640 }
                        ],
                        "notes": "循环四步。"
                    },
                    {
                        "shapes": [{ "text": "三个月", "sz": 3600, "bold": true, "color": "1C1917" }],
                        "pictures": pics("bg-chart.png"),
                        "charts": [{
                            "type": "col",
                            "title": "销量",
                            "x": 700000,
                            "y": 1850000,
                            "cx": 6400000,
                            "cy": 4100000,
                            "series": [
                                { "name": "一月", "categories": ["甲", "乙", "丙", "丁"], "values": [42, 28, 51, 19] },
                                { "name": "二月", "categories": ["甲", "乙", "丙", "丁"], "values": [38, 61, 22, 44] },
                                { "name": "三月", "categories": ["甲", "乙", "丙", "丁"], "values": [55, 33, 47, 36] }
                            ]
                        }],
                        "tables": [{
                            "rows": 2,
                            "cols": 2,
                            "x": 7400000,
                            "y": 2000000,
                            "cx": 3900000,
                            "cy": 2400000,
                            "cells": [["产品", "二月"], ["甲", "38"]]
                        }],
                        "transition": { "type": "split", "orient": "horz", "dur": 700 },
                        "anims": [
                            { "type": "fade", "picture": 4, "trigger": "with", "dur": 500 },
                            { "type": "wipe", "chart": 1, "trigger": "with", "delay": 80, "dur": 700 }
                        ],
                        "notes": "左图右表。"
                    },
                    {
                        "shapes": [
                            { "text": "回到封面", "sz": 4000, "bold": true, "color": "0F6E6E", "align": "ctr", "slide": 1 },
                            { "text": "写完了", "sz": 2000, "color": "1C1917", "align": "ctr" }
                        ],
                        "pictures": pics("bg-close.png"),
                        "anims": [
                            { "type": "fade", "shape": 1, "trigger": "with", "dur": 480 },
                            { "type": "flyIn", "picture": 2, "trigger": "with", "delay": 80, "dur": 520 }
                        ],
                        "transition": { "type": "doors", "dur": 700 },
                        "notes": "标题链回第一页。"
                    }
                ]
            }
        }).to_string();
        let cases = [
            (
                xlsx.as_str(),
                "ooxp-example-业绩.xlsx",
                r#"{"schemaVersion":1,"action":"apply","path":"ooxp-example-业绩.xlsx","ops":[{"op":"setCell","id":"cell:业绩!A1","value":"本季业绩 · 已改"},{"op":"setCell","id":"cell:业绩!C4","value":88}]}"#,
                "ooxp-example-业绩.xlsx",
            ),
            (
                docx.as_str(),
                "ooxp-example-发布说明.docx",
                r#"{"schemaVersion":1,"action":"apply","path":"ooxp-example-发布说明.docx","ops":[{"op":"setParaText","id":"para:00000001","text":"标题已经改过"}]}"#,
                "ooxp-example-发布说明.docx",
            ),
            (
                pptx.as_str(),
                "ooxp-example-发布会.pptx",
                r#"{"schemaVersion":1,"action":"apply","path":"ooxp-example-发布会.pptx","ops":[{"op":"setShapeText","id":"slide:1/shape:2","text":"ooxp · 已改"}]}"#,
                "ooxp-example-发布会.pptx",
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
