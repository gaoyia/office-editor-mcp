use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
