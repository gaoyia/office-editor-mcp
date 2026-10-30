//! `ooxp mcp`：从标准输入读 JSON-RPC，把领域 JSON 交给库。

use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

use ooxp::{envelope_fields, peek_lang, run, written_json, Action, Lang, RunOutput};
use serde_json::{json, Map, Value};

pub fn parse_lang(args: &[std::ffi::OsString]) -> Result<Option<String>, String> {
    let lang = error_lang(None);
    let only = line(
        lang,
        "mcp 只接受 --lang。例如 ooxp mcp --lang zh-CN",
        "mcp only accepts --lang. For example: ooxp mcp --lang zh-CN",
    );
    let needs = line(
        lang,
        "mcp 的 --lang 需要语言标签。例如 ooxp mcp --lang zh-CN",
        "mcp --lang needs a language tag. For example: ooxp mcp --lang zh-CN",
    );
    let Some(second) = args.get(1) else {
        return Ok(None);
    };
    let Some(text) = second.to_str() else {
        return Err(only);
    };
    let tag = if text == "--lang" {
        if args.len() != 3 {
            return Err(only);
        }
        args.get(2).and_then(|item| item.to_str())
    } else if let Some(inline) = text.strip_prefix("--lang=") {
        if args.len() != 2 {
            return Err(only);
        }
        Some(inline)
    } else {
        return Err(only);
    };
    match tag {
        Some(tag) if !tag.is_empty() => Ok(Some(tag.to_string())),
        _ => Err(needs),
    }
}

pub fn serve(default_lang: Option<String>) -> ExitCode {
    if io::stdin().is_terminal() {
        eprintln!("{}", waiting_line(default_lang.as_deref()));
    }
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    loop {
        let message = match read_message(&mut reader) {
            Ok(Some(message)) => message,
            Ok(None) => return ExitCode::SUCCESS,
            Err(_) => return ExitCode::from(1),
        };
        if let Some(response) = reply_with_lang(&message.body, default_lang.as_deref()) {
            if write_message(message.content_length, &response).is_err() {
                return ExitCode::from(1);
            }
        }
    }
}

#[cfg(test)]
pub fn reply(message: &str) -> Option<String> {
    reply_with_lang(message, None)
}

fn reply_with_lang(message: &str, default_lang: Option<&str>) -> Option<String> {
    let value: Value = match serde_json::from_str(message) {
        Ok(value) => value,
        Err(_) => {
            let lang = error_lang(default_lang);
            return Some(respond(
                Value::Null,
                Err((-32700, line(lang, "无法解析 JSON", "Cannot parse JSON"))),
            ));
        }
    };
    let id = value.get("id").filter(|id| !id.is_null()).cloned()?;
    let method = value
        .get("method")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    let params = value.get("params").cloned().unwrap_or_else(|| json!({}));
    Some(respond(id, dispatch(method, &params, default_lang)))
}

fn dispatch(
    method: &str,
    params: &Value,
    default_lang: Option<&str>,
) -> Result<Value, (i32, String)> {
    match method {
        "initialize" => Ok(initialize(params, default_lang)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(tools(chrome_lang(default_lang))),
        "tools/call" => Ok(call_tool(params, default_lang)),
        "logging/setLevel" => Ok(json!({})),
        _ => {
            let lang = error_lang(default_lang);
            let message = match lang {
                Lang::ZhCn => format!("未知方法：{method}"),
                Lang::En => format!("Unknown method: {method}"),
            };
            Err((-32601, message))
        }
    }
}

const INSTRUCTIONS_ZH: &str = "\
ooxp 是 Office Open XML Patch。它用领域 JSON 读写 docx、xlsx、pptx。真源是包里的 XML。程序按稳定 id 打补丁，其余部件按原字节写回。只有新建文件才整包生成。程序不计算公式，不排版，也不放映。

不知道如何使用这些工具时，先调用 help。不带参数可以看六个动作怎么用。要某个格式的操作名单，带上 format。要某一条操作的字段，再带上 format 和 op。查完再调用对应的工具。

调用先走 inspect，只用返回树上的 id。id 和 op 名都从 inspect 或 help 里取。接着用 apply，一次只写需要的 op。create 只用于新建文件，它是 action。op 名单里没有 create。要看它的字段，用 help 打开 name 为 createRequest 的定义。xlsx 的正文写在 workbook，docx 写在 document，pptx 写在 presentation。这三项不要同时出现。exportMedia 和 exportChartData 把字节写到 output 给出的另一个路径。这些字节不进入 inspect 树。

六个工具的参数有两种形状。直接写领域 JSON，字段名保持原样。或者只写 requestPath，指向本机上一份领域 JSON 文件。两种不要同时写。action 由工具名补上。调用方写了 action 时，它必须和工具名相同。schemaVersion 可以省略，省略视为 1。写了又不是 1，调用会被拒绝。format 可以省略，程序从 path 的扩展名推断。path 是这份 Office 文件在本机的路径。apply 要另存时，把新路径写在 output，省略则写回 path。exportMedia 和 exportChartData 把落盘路径写在 output。调用被拒绝时，读错误里的 code 和字段，再改请求。

help 一次只打开一层。它不读文件，也不改文件。不带参数时，它返回动作目录和 ask。带 format 时，它列出该格式的 op。带 q 时可以再加 format，结果会命中动作、定义和 op。带 format 和 op 时，它打开这一条操作。带 name 时，它打开一个定义。name 是定义名，例如 createRequest、inspectScope。q、op、name 不要同时出现。返回里的 kind 和 next 说明这一层是什么，以及下一步该带哪些字段。上一层的结果不是全部字段。help 不写 locale、进程也没给 --lang 时，说明是中文。";

const INSTRUCTIONS_EN: &str = "\
ooxp is Office Open XML Patch. It reads and writes docx, xlsx, and pptx with domain JSON. The source of truth is the XML in the package. The program patches that XML by stable id and writes every other part back as its original bytes. Only creating a file generates a whole package. The program does not calculate formulas, lay out pages, or run a slide show.

If you do not know how to use these tools, call help first. With no arguments it shows how the six actions work. For one format's operation list, add format. For one operation's fields, add format and op. Then call the tool.

Start with inspect, and use only ids from the returned tree. Take ids and op names from inspect or help. Then use apply, and send only the ops you need. create is only for a new file. It is an action. The op list has no create. To see its fields, open the definition named createRequest with help. The xlsx body goes in workbook, the docx body in document, and the pptx body in presentation. Do not send those three together. exportMedia and exportChartData write bytes to the path in output. Those bytes do not enter the inspect tree.

Each of the six tools takes one of two shapes. Write domain JSON directly and keep the field names. Or write only requestPath, pointing at a domain JSON file on this machine. Do not send both. The tool name fills in action. If you write action, it must match the tool name. schemaVersion may be omitted, and that means 1. Any other value is rejected. format may be omitted; the program infers it from the path extension. path is the Office file on this machine. To save apply as another file, put the new path in output. Omit it to write back to path. exportMedia and exportChartData put the destination path in output. When a call is rejected, read the error code and field, then change the request.

help opens one layer at a time. It does not read a file or change a file. With no arguments it returns the action index and ask. With format it lists that format's ops. With q it may also take format, and the hits include actions, definitions, and ops. With format and op it opens that one operation. With name it opens a definition. name is a definition name, such as createRequest or inspectScope. Do not send q, op, and name together. kind and next say what this layer is and which fields to send next. The previous layer is not the full field list. When help omits locale and the process has no --lang, the text is Chinese.";

fn initialize(params: &Value, default_lang: Option<&str>) -> Value {
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
        "instructions": instructions(default_lang)
    })
}

fn waiting_line(default_lang: Option<&str>) -> &'static str {
    say(
        chrome_lang(default_lang),
        "ooxp mcp 正在等待编辑器从标准输入发送消息。",
        "ooxp mcp is waiting for the editor to send messages on standard input.",
    )
}

fn chrome_lang(default_lang: Option<&str>) -> Lang {
    match default_lang {
        Some(tag) => Lang::parse(tag),
        None => Lang::ZhCn,
    }
}

fn say<'a>(lang: Lang, zh: &'a str, en: &'a str) -> &'a str {
    match lang {
        Lang::ZhCn => zh,
        Lang::En => en,
    }
}

fn line(lang: Lang, zh: &str, en: &str) -> String {
    say(lang, zh, en).to_string()
}

fn error_lang(default_lang: Option<&str>) -> Lang {
    Lang::resolve(
        None,
        default_lang,
        std::env::var("OOXP_LANG").ok().as_deref(),
    )
}

fn fail(_lang: Lang, code: &str, message: String, path: Option<&str>) -> Value {
    let mut error = json!({ "code": code, "message": message });
    if let Some(path) = path {
        error["path"] = json!(path);
    }
    tool_text(json!({ "ok": false, "error": error }).to_string(), true)
}

fn instructions(default_lang: Option<&str>) -> &'static str {
    say(chrome_lang(default_lang), INSTRUCTIONS_ZH, INSTRUCTIONS_EN)
}

fn tools(lang: Lang) -> Value {
    json!({
        "tools": [
            tool(
                "inspect",
                say(lang, "按 scope 看文件的一部分。范围不要一次覆盖整份文件。要改内容，改用 apply，并只用这次返回的 id。", "Read part of a file by scope. Do not cover the whole file at once. To change content, use apply, and use only ids from this result."),
                domain_properties(Action::Inspect, lang),
                &["path"],
                lang,
            ),
            tool(
                "create",
                say(lang, "只用于新建文件。已经存在的文件用 apply。xlsx 的正文写在 workbook，docx 写在 document，pptx 写在 presentation。", "Only for a new file. An existing file uses apply. The xlsx body goes in workbook, the docx body in document, and the pptx body in presentation."),
                domain_properties(Action::Create, lang),
                &["path"],
                lang,
            ),
            tool(
                "apply",
                say(lang, "按 id 和 op 修改文件里点名的内容。请求里放 ops，不要放整棵 inspect 树。某一条 op 的字段先用 help 查。", "Change named content by id and op. Put ops in the request, not the whole inspect tree. Look up one op's fields with help first."),
                domain_properties(Action::Apply, lang),
                &["path", "ops"],
                lang,
            ),
            tool(
                "exportMedia",
                say(lang, "按 id 把图、音视频、OLE、墨迹、3D 模型或嵌入字体的原字节写到 output。源文件保持不变。这些字节不进入 inspect 树。", "Write the original bytes of a picture, audio, video, OLE object, ink, 3D model, or embedded font to output by id. The source file stays unchanged. Those bytes do not enter the inspect tree."),
                domain_properties(Action::ExportMedia, lang),
                &["path", "id", "output"],
                lang,
            ),
            tool(
                "exportChartData",
                say(lang, "按图表 id 把 docx 或 pptx 图表背后的工作簿写到 output。源文件保持不变，公式不计算。xlsx 没有这个动作。", "Write the workbook behind a docx or pptx chart to output by chart id. The source file stays unchanged, and formulas are not calculated. xlsx does not have this action."),
                domain_properties(Action::ExportChartData, lang),
                &["path", "id", "output"],
                lang,
            ),
            tool(
                "help",
                say(lang, "查动作、操作和字段怎么填。一次只打开一层。format、q、op、name 的用法见返回里的 ask。这三项不要同时出现。", "Look up how to fill an action, operation, or field. Open one layer at a time. The result's ask explains format, q, op, and name. Do not send q, op, and name together."),
                domain_properties(Action::Help, lang),
                &[],
                lang,
            ),
        ]
    })
}

fn domain_properties(action: Action, lang: Lang) -> Value {
    let name = action_name(action);
    let mut properties = Map::new();
    for field in envelope_fields(action) {
        properties.insert((*field).to_string(), field_schema(name, field, lang));
    }
    Value::Object(properties)
}

fn action_name(action: Action) -> &'static str {
    match action {
        Action::Inspect => "inspect",
        Action::Create => "create",
        Action::Apply => "apply",
        Action::ExportMedia => "exportMedia",
        Action::ExportChartData => "exportChartData",
        Action::Help => "help",
    }
}

fn field_schema(action: &str, field: &str, lang: Lang) -> Value {
    match field {
        "schemaVersion" => json!({ "const": 1 }),
        "action" => json!({ "const": action }),
        "format" => json!({ "enum": ["xlsx", "xlsm", "docx", "pptx"] }),
        "locale" => json!({ "type": "string" }),
        "path" => json!({
            "type": "string",
            "minLength": 1,
            "description": say(lang, path_zh(action), path_en(action))
        }),
        "scope" => json!({
            "type": "object",
            "description": say(lang, "查看范围。字段名用 help 打开 inspectScope。", "What to read. Open inspectScope with help for the field names.")
        }),
        "workbook" | "document" | "presentation" => json!({ "type": "object" }),
        "output" => json!({
            "type": "string",
            "minLength": 1,
            "description": say(lang, output_zh(action), output_en(action))
        }),
        "ops" => json!({
            "type": "array",
            "minItems": 1,
            "items": { "type": "object" },
            "description": say(lang, "操作列表。每条的字段用 help 按 format 和 op 查。", "Operation list. Look up each item's fields with help, using format and op.")
        }),
        "id" => json!({
            "type": "string",
            "minLength": 1,
            "description": say(lang, id_zh(action), id_en(action))
        }),
        "q" | "op" | "name" => json!({ "type": "string", "minLength": 1 }),
        other => panic!("mcp 没有 {other} 的形状"),
    }
}

fn path_zh(action: &str) -> &'static str {
    match action {
        "create" => "新建文件要写到的本机路径",
        "apply" => "要修改的 Office 文件路径",
        "exportMedia" => "源 Office 文件的本机路径",
        "exportChartData" => "源 docx 或 pptx 的本机路径",
        _ => "本机上的 Office 文件路径",
    }
}

fn path_en(action: &str) -> &'static str {
    match action {
        "create" => "Local path for the new file",
        "apply" => "Path of the Office file to change",
        "exportMedia" => "Local path of the source Office file",
        "exportChartData" => "Local path of the source docx or pptx",
        _ => "Path of the Office file on this machine",
    }
}

fn output_zh(action: &str) -> &'static str {
    match action {
        "exportMedia" => "字节要写到的本机路径",
        "exportChartData" => "工作簿要写到的本机路径",
        _ => "另存路径。省略时写回 path",
    }
}

fn output_en(action: &str) -> &'static str {
    match action {
        "exportMedia" => "Local path where the bytes are written",
        "exportChartData" => "Local path where the workbook is written",
        _ => "Save-as path. Omit it to write back to path",
    }
}

fn id_zh(action: &str) -> &'static str {
    match action {
        "exportChartData" => "inspect 树上的图表 id",
        _ => "inspect 树上的图、音视频、OLE、墨迹、3D 或嵌入字体 id",
    }
}

fn id_en(action: &str) -> &'static str {
    match action {
        "exportChartData" => "Chart id on the inspect tree",
        _ => "Id of a picture, audio, video, OLE object, ink, 3D model, or embedded font on the inspect tree",
    }
}

fn tool(
    name: &str,
    description: &str,
    mut properties: Value,
    required: &[&str],
    lang: Lang,
) -> Value {
    properties["requestPath"] = json!({
        "type": "string",
        "minLength": 1,
        "description": say(
            lang,
            "本机上的领域 JSON 文件。使用时参数里只写这一个字段。",
            "A domain JSON file on this machine. When used, it is the only field."
        )
    });
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "additionalProperties": false,
            "oneOf": [
                { "required": required, "not": { "required": ["requestPath"] } },
                { "required": ["requestPath"], "maxProperties": 1 }
            ]
        }
    })
}

fn call_tool(params: &Value, default_lang: Option<&str>) -> Value {
    let name = params
        .get("name")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let lang = error_lang(default_lang);
    if !matches!(
        name,
        "inspect" | "create" | "apply" | "exportMedia" | "exportChartData" | "help"
    ) {
        let message = match lang {
            Lang::ZhCn => format!(
                "未知工具：{name}。可用的工具是 inspect、create、apply、exportMedia、exportChartData、help。"
            ),
            Lang::En => format!(
                "Unknown tool: {name}. The tools are inspect, create, apply, exportMedia, exportChartData, and help."
            ),
        };
        return fail(lang, "UnsupportedAction", message, None);
    }
    let json_text = match domain_json(name, &arguments, lang, default_lang) {
        Ok(text) => text,
        Err(error) => return error,
    };
    let lang = Lang::resolve(
        peek_lang(&json_text),
        None,
        std::env::var("OOXP_LANG").ok().as_deref(),
    );
    match run(&json_text) {
        Ok(out) => tool_text(success_text(out), false),
        Err(err) => tool_text(serde_json::to_string(&err.to_json(lang)).unwrap(), true),
    }
}

fn domain_json(
    name: &str,
    arguments: &Value,
    lang: Lang,
    default_lang: Option<&str>,
) -> Result<String, Value> {
    let Some(obj) = arguments.as_object() else {
        return Err(fail(
            lang,
            "InvalidJson",
            line(
                lang,
                "arguments 必须是对象。",
                "arguments must be an object.",
            ),
            Some("arguments"),
        ));
    };
    let source = if obj.contains_key("requestPath") {
        request_file(obj, lang)?
    } else {
        obj.clone()
    };
    compose(name, &source, lang, default_lang)
}

fn request_file(obj: &Map<String, Value>, lang: Lang) -> Result<Map<String, Value>, Value> {
    if obj.len() != 1 {
        return Err(fail(
            lang,
            "InvalidField",
            line(
                lang,
                "requestPath 与其它字段不能同时出现。",
                "requestPath cannot appear together with other fields.",
            ),
            Some("requestPath"),
        ));
    }
    let path = match obj.get("requestPath").and_then(Value::as_str) {
        Some(path) if !path.is_empty() => path,
        _ => {
            return Err(fail(
                lang,
                "InvalidField",
                line(
                    lang,
                    "requestPath 必须是非空字符串。",
                    "requestPath must be a non-empty string.",
                ),
                Some("requestPath"),
            ))
        }
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            return Err(fail(
                lang,
                "Io",
                match lang {
                    Lang::ZhCn => format!("无法读取请求文件 {path}。"),
                    Lang::En => format!("Cannot read the request file {path}."),
                },
                Some("requestPath"),
            ))
        }
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            return Err(fail(
                lang,
                "InvalidJson",
                match lang {
                    Lang::ZhCn => format!("请求文件不是 JSON：{path}"),
                    Lang::En => format!("The request file is not JSON: {path}"),
                },
                Some("requestPath"),
            ))
        }
    };
    let Some(file) = value.as_object() else {
        return Err(fail(
            lang,
            "InvalidJson",
            match lang {
                Lang::ZhCn => format!("请求文件必须是 JSON 对象：{path}"),
                Lang::En => format!("The request file must be a JSON object: {path}"),
            },
            Some("requestPath"),
        ));
    };
    if file.contains_key("requestPath") {
        return Err(fail(
            lang,
            "InvalidField",
            line(
                lang,
                "请求文件是领域 JSON，不能包含 requestPath。",
                "The request file is domain JSON and cannot contain requestPath.",
            ),
            Some("requestPath"),
        ));
    }
    Ok(file.clone())
}

fn compose(
    name: &str,
    obj: &Map<String, Value>,
    lang: Lang,
    default_lang: Option<&str>,
) -> Result<String, Value> {
    if let Some(action) = obj.get("action").and_then(Value::as_str) {
        if action != name {
            return Err(fail(
                lang,
                "InvalidField",
                match lang {
                    Lang::ZhCn => format!("action 是 {action}，这个工具是 {name}。"),
                    Lang::En => format!("action is {action}, but this tool is {name}."),
                },
                Some("action"),
            ));
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
    if let Some(tag) = default_lang {
        if !body.contains_key("locale") {
            body.insert("locale".to_string(), json!(tag));
        }
    }
    serde_json::to_string(&body).map_err(|_| {
        fail(
            lang,
            "InvalidJson",
            line(lang, "无法组成领域 JSON。", "Cannot build domain JSON."),
            None,
        )
    })
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

const MAX_FRAME: usize = 256 * 1024 * 1024;

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Incoming>> {
    loop {
        let Some(line) = read_capped_line(reader)? else {
            return Ok(None);
        };
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
        if len > MAX_FRAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "消息超过 256 MiB",
            ));
        }
        loop {
            let Some(header) = read_capped_line(reader)? else {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "缺少消息正文"));
            };
            if header.trim().is_empty() {
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

fn read_capped_line(reader: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut buf = Vec::new();
    loop {
        let next = {
            let view = reader.fill_buf()?;
            if view.is_empty() {
                None
            } else if let Some(i) = view.iter().position(|byte| *byte == b'\n') {
                Some((view[..=i].to_vec(), true))
            } else {
                Some((view.to_vec(), false))
            }
        };
        let Some((chunk, done)) = next else {
            if buf.is_empty() {
                return Ok(None);
            }
            break;
        };
        if buf.len().saturating_add(chunk.len()) > MAX_FRAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "消息超过 256 MiB",
            ));
        }
        reader.consume(chunk.len());
        buf.extend_from_slice(&chunk);
        if done {
            break;
        }
    }
    String::from_utf8(buf)
        .map(Some)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
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

    fn call_lang(name: &str, arguments: Value, lang: &str) -> Value {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        let response = reply_with_lang(&request.to_string(), Some(lang)).unwrap();
        serde_json::from_str(&response).unwrap()
    }

    fn tool_message(response: &Value) -> &str {
        response["result"]["content"][0]["text"].as_str().unwrap()
    }

    fn error_json(response: &Value) -> Value {
        serde_json::from_str(tool_message(response)).unwrap()
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
        let instructions = parsed["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("不知道如何使用这些工具时，先调用 help"));
        assert!(instructions.contains("Office Open XML Patch"));
        assert!(instructions.contains("createRequest"));
        assert!(instructions.contains("inspectScope"));
        assert!(instructions.contains("kind"));
        assert!(instructions.contains("requestPath"));
    }

    #[test]
    fn initialize_and_tools_follow_mcp_lang() {
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}"#;
        let english = reply_with_lang(request, Some("en")).unwrap();
        let english: Value = serde_json::from_str(&english).unwrap();
        let instructions = english["result"]["instructions"].as_str().unwrap();
        assert!(instructions.contains("call help first"));
        assert!(instructions.contains("requestPath"));
        assert!(!instructions.contains("不知道如何使用"));

        let chinese = reply_with_lang(request, Some("zh")).unwrap();
        let chinese: Value = serde_json::from_str(&chinese).unwrap();
        assert!(chinese["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("不知道如何使用这些工具时，先调用 help"));

        let listed = reply_with_lang(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            Some("en"),
        )
        .unwrap();
        let listed: Value = serde_json::from_str(&listed).unwrap();
        assert!(listed["result"]["tools"][0]["description"]
            .as_str()
            .unwrap()
            .contains("Read part of a file by scope"));
    }

    #[test]
    fn unknown_tool_uses_the_error_shape() {
        let response = call_lang("nope", json!({}), "en");
        assert_eq!(response["result"]["isError"].as_bool(), Some(true));
        let err = error_json(&response);
        assert_eq!(err["error"]["code"], "UnsupportedAction");
        assert!(err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unknown tool: nope"));
        let response = reply_with_lang(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"help","arguments":{"requestPath":"x.json","format":"xlsx"}}}"#,
            Some("en"),
        )
        .unwrap();
        let response: Value = serde_json::from_str(&response).unwrap();
        let err = error_json(&response);
        assert_eq!(err["error"]["code"], "InvalidField");
        assert!(err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cannot appear together"));
    }

    #[test]
    fn lang_flag_is_optional() {
        let args = [std::ffi::OsString::from("mcp")];
        assert_eq!(parse_lang(&args).unwrap(), None);
        let args = [
            std::ffi::OsString::from("mcp"),
            std::ffi::OsString::from("--lang"),
            std::ffi::OsString::from("zh-CN"),
        ];
        assert_eq!(parse_lang(&args).unwrap().as_deref(), Some("zh-CN"));
        let args = [
            std::ffi::OsString::from("mcp"),
            std::ffi::OsString::from("--pretty"),
        ];
        assert!(parse_lang(&args).is_err());
        assert!(waiting_line(Some("en")).starts_with("ooxp mcp is waiting"));
        assert!(waiting_line(None).contains("正在等待"));
        assert!(waiting_line(Some("zh")).contains("正在等待"));
    }

    #[test]
    fn default_lang_translates_help_until_locale_overrides_it() {
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"help","arguments":{"format":"xlsx","op":"setCell"}}}"#;
        let english = reply_with_lang(request, Some("en")).unwrap();
        let english: Value = serde_json::from_str(&english).unwrap();
        let text = english["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Change a cell's value or formula."));

        let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"help","arguments":{"format":"xlsx","op":"setCell","locale":"zh-CN"}}}"#;
        let chinese = reply_with_lang(request, Some("en")).unwrap();
        let chinese: Value = serde_json::from_str(&chinese).unwrap();
        let text = chinese["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("改一个单元格的值或公式。"));
    }

    #[test]
    fn english_help_index_uses_an_english_example() {
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"help","arguments":{}}}"#;
        let english = reply_with_lang(request, Some("en")).unwrap();
        let english: Value = serde_json::from_str(&english).unwrap();
        let text = english["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("\"q\":\"cell\""));
        assert!(!text.contains("单元格"));
        assert!(!text.contains('。'));
    }

    #[test]
    fn request_path_loads_the_domain_json_file() {
        let path = std::env::temp_dir().join(format!("ooxp-mcp-req-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"format":"xlsx","op":"setCell"}"#).unwrap();
        let path_text = path.display().to_string();
        let response = call("help", json!({ "requestPath": path_text }));
        let _ = std::fs::remove_file(&path);
        assert!(response["result"].get("isError").is_none());
        assert!(tool_message(&response).contains("改一个单元格的值或公式。"));
    }

    #[test]
    fn request_path_rejects_other_fields_and_a_mismatched_action() {
        let mixed = call_lang(
            "help",
            json!({ "requestPath": "ignored.json", "format": "xlsx" }),
            "zh-CN",
        );
        assert_eq!(mixed["result"]["isError"].as_bool(), Some(true));
        let mixed_err = error_json(&mixed);
        assert_eq!(mixed_err["error"]["code"], "InvalidField");
        assert!(mixed_err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("不能同时出现"));

        let missing = call_lang(
            "help",
            json!({ "requestPath": "ooxp-mcp-no-such-request.json" }),
            "zh-CN",
        );
        assert_eq!(missing["result"]["isError"].as_bool(), Some(true));
        let missing_err = error_json(&missing);
        assert_eq!(missing_err["error"]["code"], "Io");
        assert!(missing_err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("无法读取请求文件"));

        let path = std::env::temp_dir().join(format!("ooxp-mcp-act-{}.json", std::process::id()));
        std::fs::write(
            &path,
            r#"{"action":"inspect","format":"xlsx","op":"setCell"}"#,
        )
        .unwrap();
        let response = call_lang(
            "help",
            json!({ "requestPath": path.display().to_string() }),
            "zh-CN",
        );
        let _ = std::fs::remove_file(&path);
        assert_eq!(response["result"]["isError"].as_bool(), Some(true));
        let err = error_json(&response);
        assert_eq!(err["error"]["code"], "InvalidField");
        assert!(err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("这个工具是 help"));
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

    #[test]
    fn content_length_over_256_mib_is_rejected_before_alloc() {
        let header = format!("Content-Length: {}\r\n\r\n", 256 * 1024 * 1024 + 1);
        let mut cursor = std::io::Cursor::new(header.into_bytes());
        let err = match read_message(&mut cursor) {
            Err(err) => err,
            Ok(_) => panic!("超长 Content-Length 应被拒绝"),
        };
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }
}
