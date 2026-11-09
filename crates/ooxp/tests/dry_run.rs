//! apply.dryRun：不写盘、忽略 output、列出标脏部件（含被删部件），失败时挂对象字段。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ooxp::{run, Lang, RunOutput};
use serde_json::{json, Value};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("ooxp-dry-run-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{tag}-{n}.xlsx"))
}

fn create_book() -> PathBuf {
    let path = tmp("src");
    run(&json!({
        "schemaVersion": 1,
        "action": "create",
        "path": path.to_string_lossy(),
        "workbook": { "sheets": [{"name": "A"}, {"name": "B"}] }
    })
    .to_string())
    .unwrap();
    path
}

fn apply_request(src: &Path, dry: bool, op: Value) -> String {
    let mut request = json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "ops": [op],
    });
    if dry {
        request["dryRun"] = json!(true);
    }
    request.to_string()
}

#[test]
fn dry_run_ignores_output_and_lists_removed_parts() {
    let src = create_book();
    let before = std::fs::read(&src).unwrap();
    let output = tmp("never-written");
    let _ = std::fs::remove_file(&output);

    let request = json!({
        "schemaVersion": 1,
        "action": "apply",
        "path": src.to_string_lossy(),
        "output": output.to_string_lossy(),
        "dryRun": true,
        "ops": [{"op": "deleteSheet", "id": "sheet:B"}],
    });
    let out = run(&request.to_string()).unwrap();
    let RunOutput::DryRun { ops_applied, parts } = out else {
        panic!("dryRun 应返回 DryRun");
    };
    assert_eq!(ops_applied, 1);
    // deleteSheet 会 remove 工作表部件；删除也算标脏，parts 必须列出它。
    assert!(
        parts.contains(&"/xl/worksheets/sheet2.xml".to_string()),
        "被删部件没进 parts: {parts:?}"
    );
    let mut sorted = parts.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(parts, sorted, "parts 必须排序去重");
    assert!(!output.exists(), "dryRun 不能写出 output");
    assert_eq!(std::fs::read(&src).unwrap(), before, "dryRun 不能改源包");
}

#[test]
fn apply_and_dry_run_failures_carry_object_fields() {
    let src = create_book();
    for dry in [false, true] {
        let err = run(&apply_request(
            &src,
            dry,
            json!({"op": "deleteSheet", "id": "sheet:Nope", "name": "不存在的表"}),
        ))
        .unwrap_err();
        let value = err.to_json(Lang::En);
        assert!(
            value["error"]["path"]
                .as_str()
                .unwrap()
                .starts_with("ops[0]."),
            "parse_ops 期错误: {value:?}"
        );
        assert_eq!(value["error"]["objectId"], "sheet:Nope", "{value:?}");
        assert_eq!(value["error"]["objectName"], "不存在的表", "{value:?}");

        // 执行期错误（找不到表）同样挂 objectId；op 不带 name 就该省略 objectName。
        let err = run(&apply_request(
            &src,
            dry,
            json!({"op": "deleteSheet", "id": "sheet:Nope"}),
        ))
        .unwrap_err();
        let value = err.to_json(Lang::En);
        assert_eq!(value["error"]["path"], "ops[0].id", "{value:?}");
        assert_eq!(value["error"]["objectId"], "sheet:Nope", "{value:?}");
        assert!(value["error"].get("objectName").is_none(), "{value:?}");
    }
}

#[test]
fn non_string_id_is_not_reported_as_object_id() {
    let src = create_book();
    let err = run(&apply_request(
        &src,
        false,
        json!({"op": "setCell", "id": 12, "value": 1}),
    ))
    .unwrap_err();
    let value = err.to_json(Lang::En);
    assert_eq!(value["error"]["path"], "ops[0].id", "{value:?}");
    assert!(
        value["error"].get("objectId").is_none(),
        "数字 id 不该当 objectId: {value:?}"
    );
}
