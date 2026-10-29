//! `ooxp mcp`：从标准输入读 JSON-RPC，把领域 JSON 交给库。

use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

use ooxp::{peek_lang, run, written_json, Lang, RunOutput};
use serde_json::{json, Map, Value};

pub fn serve() -> ExitCode {
    if io::stdin().is_terminal() {
        eprintln!("ooxp mcp 正在等待编辑器从标准输入发送消息。");
    }
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    loop {
        let message = match read_message(&mut reader) {
            Ok(Some(message)) => message,
            Ok(None) => return ExitCode::SUCCESS,
            Err(_) => return ExitCode::from(1),
        };
        if let Some(response) = reply(&message.body) {
            if write_message(message.content_length, &response).is_err() {
                return ExitCode::from(1);
            }
        }
    }
}

pub fn reply(message: &str) -> Option<String> {
    let value: Value = match serde_json::from_str(message) {
        Ok(value) => value,
        Err(_) => {
            return Some(respond(
                Value::Null,
                Err((-32700, "无法解析 JSON".to_string())),
            ));
        }
    };
    let id = value.get("id").filter(|id| !id.is_null()).cloned()?;
    let method = value
        .get("method")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    let params = value.get("params").cloned().unwrap_or_else(|| json!({}));
    Some(respond(id, dispatch(method, &params)))
}

fn dispatch(method: &str, params: &Value) -> Result<Value, (i32, String)> {
    match method {
        "initialize" => Ok(initialize(params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools()),
        "tools/call" => Ok(call_tool(params)),
        "logging/setLevel" => Ok(json!({})),
        _ => Err((-32601, format!("未知方法: {method}"))),
    }
}

fn initialize(params: &Value) -> Value {
    let version = params
        .get("protocolVersion")
        .and_then(|item| item.as_str())
        .filter(|version| !version.is_empty())
        .unwrap_or("2024-11-05");
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": {
            "name": "ooxp",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "用领域 JSON 读写 docx、xlsx、pptx。六个工具的参数就是领域 JSON，不要另造字段。操作名单很长，先调用 help，再调用 apply。path 和 output 写在参数里。"
    })
}

fn tools() -> Value {
    json!({
        "tools": [
            tool(
                "inspect",
                "按 scope 看文件的一部分。不要一次要整份文件。改内容用 apply，不要把看到的树整份送回去。",
                json!({
                    "path": { "type": "string", "minLength": 1, "description": "本地文件路径" },
                    "format": { "enum": ["xlsx", "xlsm", "docx", "pptx"] },
                    "scope": { "type": "object", "description": "查看范围。键用 help 的 name=inspectScope 查。" },
                    "locale": { "type": "string" },
                    "schemaVersion": { "const": 1 },
                    "action": { "const": "inspect" }
                }),
                &["path"],
            ),
            tool(
                "create",
                "只用于新建文件。已经存在的文件用 apply。xlsx 用 workbook，docx 用 document，pptx 用 presentation，三者不要同时写。",
                json!({
                    "path": { "type": "string", "minLength": 1, "description": "要写出的本地路径" },
                    "format": { "enum": ["xlsx", "xlsm", "docx", "pptx"] },
                    "workbook": { "type": "object" },
                    "document": { "type": "object" },
                    "presentation": { "type": "object" },
                    "locale": { "type": "string" },
                    "schemaVersion": { "const": 1 },
                    "action": { "const": "create" }
                }),
                &["path"],
            ),
            tool(
                "apply",
                "用 id 和 op 改文件里点名的内容。不要把整棵 inspect 树塞进请求。某一条 op 怎么填，先调用 help。",
                json!({
                    "path": { "type": "string", "minLength": 1, "description": "源文件路径" },
                    "format": { "enum": ["xlsx", "xlsm", "docx", "pptx"] },
                    "output": { "type": "string", "minLength": 1, "description": "另存路径。省略则写回 path" },
                    "ops": {
                        "type": "array",
                        "minItems": 1,
                        "items": { "type": "object" },
                        "description": "操作列表。每条的形状用 help 按 format 和 op 查。"
                    },
                    "locale": { "type": "string" },
                    "schemaVersion": { "const": 1 },
                    "action": { "const": "apply" }
                }),
                &["path", "ops"],
            ),
            tool(
                "exportMedia",
                "按 id 把图或嵌入对象的原始字节写到本地。不改源文件，字节也不进 inspect 树。",
                json!({
                    "path": { "type": "string", "minLength": 1 },
                    "format": { "enum": ["xlsx", "xlsm", "docx", "pptx"] },
                    "id": { "type": "string", "minLength": 1 },
                    "output": { "type": "string", "minLength": 1, "description": "写出路径" },
                    "locale": { "type": "string" },
                    "schemaVersion": { "const": 1 },
                    "action": { "const": "exportMedia" }
                }),
                &["path", "id", "output"],
            ),
            tool(
                "exportChartData",
                "按图表 id 把背后的工作簿数据导出。不改源文件，不算公式。",
                json!({
                    "path": { "type": "string", "minLength": 1 },
                    "format": { "enum": ["xlsx", "xlsm", "docx", "pptx"] },
                    "id": { "type": "string", "minLength": 1 },
                    "output": { "type": "string", "minLength": 1, "description": "写出路径" },
                    "locale": { "type": "string" },
                    "schemaVersion": { "const": 1 },
                    "action": { "const": "exportChartData" }
                }),
                &["path", "id", "output"],
            ),
            tool(
                "help",
                "查某个操作怎么填。一次只打开一层。format、q、op、name 的用法见返回里的 ask。q、op、name 不要同时写。",
                json!({
                    "format": { "enum": ["xlsx", "xlsm", "docx", "pptx"] },
                    "q": { "type": "string", "minLength": 1 },
                    "op": { "type": "string", "minLength": 1 },
                    "name": { "type": "string", "minLength": 1 },
                    "locale": { "type": "string" },
                    "schemaVersion": { "const": 1 },
                    "action": { "const": "help" }
                }),
                &[],
            ),
        ]
    })
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        }
    })
}

fn call_tool(params: &Value) -> Value {
    let name = params
        .get("name")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !matches!(
        name,
        "inspect" | "create" | "apply" | "exportMedia" | "exportChartData" | "help"
    ) {
        return tool_error(format!(
            "未知工具: {name}。可用的工具是 inspect、create、apply、exportMedia、exportChartData、help。"
        ));
    }
    let json_text = match domain_json(name, &arguments) {
        Ok(text) => text,
        Err(error) => return tool_error(error),
    };
    let lang = Lang::resolve(
        peek_lang(&json_text),
        None,
        std::env::var("OOXP_LANG").ok().as_deref(),
    );
    match run(&json_text, None) {
        Ok(out) => tool_text(success_text(out), false),
        Err(err) => tool_text(serde_json::to_string(&err.to_json(lang)).unwrap(), true),
    }
}

fn domain_json(name: &str, arguments: &Value) -> Result<String, String> {
    let Some(obj) = arguments.as_object() else {
        return Err("arguments 必须是对象。".to_string());
    };
    if let Some(action) = obj.get("action").and_then(Value::as_str) {
        if action != name {
            return Err(format!("action 是 {action}，这个工具是 {name}。"));
        }
    }
    let mut body = Map::new();
    body.insert("action".to_string(), json!(name));
    for (key, value) in obj {
        if key == "action" {
            continue;
        }
        body.insert(key.clone(), value.clone());
    }
    serde_json::to_string(&body).map_err(|_| "无法组成领域 JSON。".to_string())
}

fn success_text(out: RunOutput) -> String {
    match out {
        RunOutput::Inspect(tree) | RunOutput::Help(tree) => serde_json::to_string(&tree).unwrap(),
        RunOutput::Written {
            path,
            ops_applied,
            content_type,
        } => written_json(&path, ops_applied, content_type.as_deref(), false),
    }
}

fn tool_text(text: String, is_error: bool) -> Value {
    let mut result = json!({
        "content": [{ "type": "text", "text": text }]
    });
    if is_error {
        result["isError"] = json!(true);
    }
    result
}

fn tool_error(text: String) -> Value {
    tool_text(text, true)
}

fn respond(id: Value, result: Result<Value, (i32, String)>) -> String {
    let message = match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": code, "message": message }
        }),
    };
    message.to_string()
}

struct Incoming {
    body: String,
    content_length: bool,
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Incoming>> {
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('{') {
            return Ok(Some(Incoming {
                body: trimmed.to_string(),
                content_length: false,
            }));
        }
        let Some(len_text) = trimmed.strip_prefix("Content-Length:") else {
            continue;
        };
        let len: usize = len_text
            .trim()
            .parse()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "缺少消息正文"));
            }
            if line.trim().is_empty() {
                break;
            }
        }
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        let body = String::from_utf8(buf)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        return Ok(Some(Incoming {
            body,
            content_length: true,
        }));
    }
}

fn write_message(content_length: bool, body: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    if content_length {
        write!(out, "Content-Length: {}\r\n\r\n", body.len())?;
        out.write_all(body.as_bytes())?;
    } else {
        out.write_all(body.as_bytes())?;
        out.write_all(b"\n")?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, arguments: Value) -> Value {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        let response = reply(&request.to_string()).unwrap();
        serde_json::from_str(&response).unwrap()
    }

    fn tool_message(response: &Value) -> &str {
        response["result"]["content"][0]["text"].as_str().unwrap()
    }

    #[test]
    fn initialize_names_the_server() {
        let response = reply(
            r#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}"#,
        )
        .unwrap();
        let parsed: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed["id"].as_i64(), Some(7));
        assert_eq!(
            parsed["result"]["serverInfo"]["name"].as_str(),
            Some("ooxp")
        );
        assert_eq!(
            parsed["result"]["protocolVersion"].as_str(),
            Some("2024-11-05")
        );
    }

    #[test]
    fn initialized_notification_has_no_reply() {
        assert!(reply(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    }

    #[test]
    fn tools_are_the_six_actions() {
        let response = reply(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
        let parsed: Value = serde_json::from_str(&response).unwrap();
        let names: Vec<_> = parsed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "inspect",
                "create",
                "apply",
                "exportMedia",
                "exportChartData",
                "help"
            ]
        );
    }

    #[test]
    fn help_omits_schema_version() {
        let response = call("help", json!({ "format": "xlsx", "op": "setCell" }));
        assert!(response.get("error").is_none());
        assert!(response["result"].get("isError").is_none());
        assert!(tool_message(&response).contains("setCell"));
    }

    #[test]
    fn schema_version_two_is_a_tool_error() {
        let response = call("help", json!({ "schemaVersion": 2, "op": "setCell" }));
        assert_eq!(response["result"]["isError"].as_bool(), Some(true));
        let body: Value = serde_json::from_str(tool_message(&response)).unwrap();
        assert_eq!(body["error"]["code"], "UnsupportedSchema");
    }

    #[test]
    fn action_must_match_the_tool() {
        let response = call("help", json!({ "action": "inspect" }));
        assert_eq!(response["result"]["isError"].as_bool(), Some(true));
        assert!(tool_message(&response).contains("inspect"));
    }

    #[test]
    fn create_then_inspect_a_cell() {
        let path = std::env::temp_dir().join(format!("ooxp-mcp-{}.xlsx", std::process::id()));
        let path_text = path.display().to_string();
        let created = call(
            "create",
            json!({
                "path": path_text,
                "workbook": {
                    "sheets": [{
                        "name": "Sheet1",
                        "cells": [{ "ref": "A1", "value": 1 }]
                    }]
                }
            }),
        );
        assert!(
            created["result"].get("isError").is_none(),
            "{}",
            tool_message(&created)
        );
        let inspected = call(
            "inspect",
            json!({
                "path": path_text,
                "scope": { "sheet": "Sheet1", "range": "A1:A1" }
            }),
        );
        assert!(inspected["result"].get("isError").is_none());
        assert!(tool_message(&inspected).contains("cell:Sheet1!A1"));
        let _ = std::fs::remove_file(path);
    }
}
