use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-cli-tests");
    let _ = fs::create_dir_all(&dir);
    dir.join(name)
}

#[test]
fn cli_create_then_inspect() {
    let bin = env!("CARGO_BIN_EXE_ooxp");
    let xlsx = tmp("cli-book.xlsx");
    let req = tmp("create.json");
    fs::write(
        &req,
        format!(
            r#"{{"schemaVersion":1,"action":"create","path":{},"workbook":{{"sheets":[{{"name":"Sheet1","cells":[{{"ref":"A1","value":1}}]}}]}}}}"#,
            serde_json::Value::String(xlsx.to_string_lossy().into_owned())
        ),
    )
    .unwrap();

    let out = Command::new(bin)
        .args(["--input", req.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let inspect = tmp("inspect.json");
    fs::write(
        &inspect,
        format!(
            r#"{{"schemaVersion":1,"action":"inspect","path":{},"scope":{{"sheet":"Sheet1","range":"A1:A1"}}}}"#,
            serde_json::Value::String(xlsx.to_string_lossy().into_owned())
        ),
    )
    .unwrap();
    let out = Command::new(bin)
        .args(["--input", inspect.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("cell:Sheet1!A1"));
    assert!(stdout.contains("\"value\":1"));
}

fn run_fail(args: &[&str], env_lang: Option<&str>, stdin: &str) -> (i32, serde_json::Value) {
    let bin = env!("CARGO_BIN_EXE_ooxp");
    let mut cmd = Command::new(bin);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_remove("OOXP_LANG");
    if let Some(v) = env_lang {
        cmd.env("OOXP_LANG", v);
    }
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let code = out.status.code().unwrap_or(-1);
    // 严格解码，不用 lossy：stderr 必须是 UTF-8，管道场景不受控制台代码页影响。
    // 见 `docs/cli.md` 的「编码与路径」。
    let stderr = String::from_utf8(out.stderr).expect("stderr 不是 UTF-8");
    let json: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap_or_else(|_| {
        panic!("stderr 不是 JSON：{stderr}");
    });
    (code, json)
}

#[test]
fn mcp_initialize_speaks_jsonrpc() {
    let bin = env!("CARGO_BIN_EXE_ooxp");
    let mut child = Command::new(bin)
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}
"#,
        )
        .unwrap();
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(json["result"]["serverInfo"]["name"], "ooxp");
}

#[test]
fn error_default_is_english() {
    let (code, json) = run_fail(
        &["--input", "-"],
        None,
        r#"{"schemaVersion":2,"action":"inspect","path":"x.xlsx"}"#,
    );
    assert_eq!(code, 2);
    assert_eq!(json["error"]["code"], "UnsupportedSchema");
    assert_eq!(json["error"]["message"], "schemaVersion must be 1");
}

#[test]
fn locale_beats_lang_flag() {
    let (code, json) = run_fail(
        &["--input", "-", "--lang", "en"],
        Some("en"),
        r#"{"schemaVersion":2,"action":"inspect","path":"x.xlsx","locale":"zh-CN"}"#,
    );
    assert_eq!(code, 2);
    assert_eq!(json["error"]["message"], "schemaVersion 必须是 1");
}

#[test]
fn lang_flag_beats_env() {
    let (code, json) = run_fail(
        &["--input", "-", "--lang", "zh-CN"],
        Some("en"),
        r#"{"schemaVersion":2,"action":"inspect","path":"x.xlsx"}"#,
    );
    assert_eq!(code, 2);
    assert_eq!(json["error"]["message"], "schemaVersion 必须是 1");
}

#[test]
fn env_lang_used_when_no_locale_or_flag() {
    let (code, json) = run_fail(
        &["--input", "-"],
        Some("zh-Hans"),
        r#"{"schemaVersion":2,"action":"inspect","path":"x.xlsx"}"#,
    );
    assert_eq!(code, 2);
    assert_eq!(json["error"]["message"], "schemaVersion 必须是 1");
}

#[test]
fn output_flag_is_rejected() {
    let bin = env!("CARGO_BIN_EXE_ooxp");
    let out = Command::new(bin)
        .args(["--output", "x.xlsx", "--input", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn create_without_path_reports_missing_path() {
    let (code, json) = run_fail(
        &["--input", "-"],
        None,
        r#"{"schemaVersion":1,"action":"create","workbook":{"sheets":[{"name":"Sheet1"}]}}"#,
    );
    assert_eq!(code, 2);
    assert_eq!(json["error"]["code"], "InvalidField");
    assert_eq!(json["error"]["message"], "Missing path");
    assert_eq!(json["error"]["path"], "path");
}

#[test]
fn export_without_output_reports_missing_output() {
    let (code, json) = run_fail(
        &["--input", "-"],
        None,
        r#"{"schemaVersion":1,"action":"exportMedia","path":"a.docx","id":"para:1/pic:1"}"#,
    );
    assert_eq!(code, 2);
    assert_eq!(json["error"]["code"], "InvalidField");
    assert_eq!(json["error"]["message"], "Missing output");
}

#[test]
fn apply_without_output_overwrites_path() {
    let bin = env!("CARGO_BIN_EXE_ooxp");
    let xlsx = tmp("cli-overwrite.xlsx");
    let _ = fs::remove_file(&xlsx);
    let path = serde_json::Value::String(xlsx.to_string_lossy().into_owned());
    let create = format!(
        r#"{{"schemaVersion":1,"action":"create","path":{path},"workbook":{{"sheets":[{{"name":"Sheet1","cells":[{{"ref":"A1","value":1}}]}}]}}}}"#
    );
    let mut child = Command::new(bin)
        .args(["--input", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(create.as_bytes())
        .unwrap();
    let created = child.wait_with_output().unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let apply = format!(
        r#"{{"schemaVersion":1,"action":"apply","path":{path},"ops":[{{"op":"setCell","id":"cell:Sheet1!A1","value":2}}]}}"#
    );
    let mut child = Command::new(bin)
        .args(["--input", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(apply.as_bytes())
        .unwrap();
    let applied = child.wait_with_output().unwrap();
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let stdout = String::from_utf8(applied.stdout).unwrap();
    assert!(stdout.contains("cli-overwrite.xlsx"));
}

#[test]
fn unknown_locale_falls_back_to_english() {
    let (code, json) = run_fail(
        &["--input", "-"],
        None,
        r#"{"schemaVersion":2,"action":"inspect","path":"x.xlsx","locale":"ja"}"#,
    );
    assert_eq!(code, 2);
    assert_eq!(json["error"]["message"], "schemaVersion must be 1");
}
